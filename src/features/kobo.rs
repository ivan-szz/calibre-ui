//! Kobo sync: the store API that Kobo e-readers talk to, in the form calibre-web
//! implements. A device is pointed at `{PUBLIC_URL}/kobo/{token}` in its
//! `eReader.conf`, then syncs the user's EPUB and KEPUB books and their reading state.

use base64::Engine as _;
use serde_json::{Value, json};
use topcoat::{Result, context::Cx};

use super::{library::Book, shelf::ReadMark};
use crate::db::db;

/// The last reading state a device sent for a book, kept as the device sent it.
#[derive(Debug, toasty::Model)]
pub struct KoboState {
    #[key]
    #[auto]
    pub id: u64,

    #[index]
    pub user_id: u64,

    #[index]
    pub book_id: u64,

    /// One `ReadingStates` entry from the device, as JSON.
    pub state: String,
}

/// Calibre's stored `2024-01-10 10:00:00.123+00:00` as Kobo's `2024-01-10T10:00:00Z`.
pub fn timestamp(stored: &str) -> String {
    match stored.get(..19) {
        Some(t) => format!("{}Z", t.replacen(' ', "T", 1)),
        None => "1970-01-01T00:00:00Z".to_owned(),
    }
}

/// The `x-kobo-synctoken` header for the newest change a device has received.
pub fn sync_token(books_last_modified: &str) -> String {
    let json = json!({ "books_last_modified": books_last_modified }).to_string();
    base64::engine::general_purpose::STANDARD.encode(json)
}

/// The newest change in a device's `x-kobo-synctoken`, or empty for a first sync. A
/// token from Kobo's own store, or garbage, also starts over.
pub fn read_sync_token(header: Option<&str>) -> String {
    header
        .and_then(|h| base64::engine::general_purpose::STANDARD.decode(h.trim()).ok())
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|v| v.get("books_last_modified")?.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// The download links of a book, KEPUB first, as Kobo devices pick the first match.
fn download_urls(book: &Book, base: &str) -> Vec<Value> {
    let mut formats: Vec<_> = book.formats.iter().filter(|f| f.format == "KEPUB" || f.format == "EPUB").collect();
    formats.sort_by_key(|f| f.format != "KEPUB");
    formats
        .into_iter()
        .map(|f| {
            json!({
                "Format": if f.format == "KEPUB" { "KEPUB" } else { "EPUB3" },
                "Size": f.size,
                "Url": format!("{base}/download/{}/{}", book.id, f.format.to_lowercase()),
                "Platform": "Generic",
            })
        })
        .collect()
}

/// The `BookMetadata` of a book. `base` is the device's sync URL, `…/kobo/{token}`.
pub fn metadata(book: &Book, base: &str) -> Value {
    let language = book.languages.first().map_or("en".to_owned(), |l| iso639_1(&l.name).to_owned());
    let mut meta = json!({
        "Categories": ["00000000-0000-0000-0000-000000000001"],
        "CoverImageId": book.uuid,
        "CrossRevisionId": book.uuid,
        "CurrentDisplayPrice": { "CurrencyCode": "USD", "TotalAmount": 0 },
        "CurrentLoveDisplayPrice": { "TotalAmount": 0 },
        "Description": book.description.iter().map(|p| format!("<p>{}</p>", html_escape(p))).collect::<String>(),
        "DownloadUrls": download_urls(book, base),
        "EntitlementId": book.uuid,
        "ExternalIds": [],
        "Genre": "00000000-0000-0000-0000-000000000001",
        "IsEligibleForKoboLove": false,
        "IsInternetArchive": false,
        "IsPreOrder": false,
        "IsSocialEnabled": true,
        "Language": language,
        "PhoneticPronunciations": {},
        "PublicationDate": if book.pubdate.is_empty() { Value::Null } else { timestamp(&book.pubdate).into() },
        "Publisher": { "Imprint": "", "Name": book.publishers.first().map(|p| p.name.clone()) },
        "RevisionId": book.uuid,
        "Title": book.title,
        "WorkId": book.uuid,
        "ContributorRoles": book.authors.iter().map(|a| json!({ "Name": a.name })).collect::<Vec<_>>(),
        "Contributors": book.authors.iter().map(|a| a.name.clone()).collect::<Vec<_>>(),
    });
    if let Some(series) = book.series.first() {
        meta["Series"] = json!({
            "Name": series.name,
            "Number": book.series_index.trunc() as i64,
            "NumberFloat": book.series_index,
            // A stable id per series name.
            "Id": format!("series-{}", series.id),
        });
    }
    meta
}

/// The `BookEntitlement` of a book.
fn entitlement(book: &Book) -> Value {
    json!({
        "Accessibility": "Full",
        "ActivePeriod": { "From": timestamp(&book.added) },
        "Created": timestamp(&book.added),
        "CrossRevisionId": book.uuid,
        "Id": book.uuid,
        "IsRemoved": false,
        "IsHiddenFromArchive": false,
        "IsLocked": false,
        "LastModified": timestamp(&book.last_modified),
        "OriginCategory": "Imported",
        "RevisionId": book.uuid,
        "Status": "Active",
    })
}

/// One `NewEntitlement` of a sync response, with the reading state if there is one.
pub fn new_entitlement(book: &Book, base: &str, state: Option<Value>) -> Value {
    let mut entitlement = json!({ "BookEntitlement": entitlement(book), "BookMetadata": metadata(book, base) });
    if let Some(state) = state {
        entitlement["ReadingState"] = state;
    }
    json!({ "NewEntitlement": entitlement })
}

/// The state of a book nobody has opened yet.
fn unread_state(book: &Book) -> Value {
    let t = timestamp(&book.last_modified);
    json!({
        "EntitlementId": book.uuid,
        "Created": t,
        "LastModified": t,
        "PriorityTimestamp": t,
        "StatusInfo": { "LastModified": t, "Status": "ReadyToRead", "TimesStartedReading": 0 },
        "Statistics": { "LastModified": t },
        "CurrentBookmark": { "LastModified": t },
    })
}

impl KoboState {
    /// The last state the user's device sent for a book.
    pub async fn get(cx: &Cx, user_id: u64, book: &Book) -> Result<Option<Value>> {
        let state = KoboState::filter_by_user_id(user_id)
            .filter(KoboState::fields().book_id().eq(book.id as u64))
            .first()
            .exec(&mut db(cx))
            .await?;
        Ok(state.and_then(|s| serde_json::from_str(&s.state).ok()))
    }

    /// The state to give a device: the stored one, or a fresh unread one.
    pub async fn current(cx: &Cx, user_id: u64, book: &Book) -> Result<Value> {
        Ok(Self::get(cx, user_id, book).await?.unwrap_or_else(|| unread_state(book)))
    }

    /// Stores a state the device sent. Finishing a book marks it read, as calibre-web
    /// does.
    pub async fn save(cx: &Cx, user_id: u64, book: &Book, mut state: Value) -> Result<()> {
        state["EntitlementId"] = book.uuid.clone().into();
        let finished = state.pointer("/StatusInfo/Status").and_then(Value::as_str) == Some("Finished");
        let mut db = db(cx);
        KoboState::filter_by_user_id(user_id)
            .filter(KoboState::fields().book_id().eq(book.id as u64))
            .delete()
            .exec(&mut db)
            .await?;
        toasty::create!(KoboState { user_id, book_id: book.id as u64, state: state.to_string() }).exec(&mut db).await?;
        let read = ReadMark::book_ids(cx, user_id).await?.contains(&(book.id as u64));
        if finished && !read {
            ReadMark::toggle(cx, user_id, book.id as u64).await?;
        }
        Ok(())
    }
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Kobo wants two-letter language codes; Calibre stores three-letter ones.
fn iso639_1(code: &str) -> &str {
    match code {
        "eng" => "en",
        "ita" => "it",
        "fra" | "fre" => "fr",
        "deu" | "ger" => "de",
        "spa" => "es",
        "por" => "pt",
        "nld" | "dut" => "nl",
        "jpn" => "ja",
        "zho" | "chi" => "zh",
        "rus" => "ru",
        other if other.len() == 2 => other,
        _ => "en",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_sync_tokens() {
        let token = sync_token("2024-01-10 10:00:00+00:00");
        assert_eq!(read_sync_token(Some(&token)), "2024-01-10 10:00:00+00:00");
        assert_eq!(read_sync_token(Some("not base64!")), "");
        assert_eq!(read_sync_token(None), "");
    }

    #[test]
    fn formats_timestamps() {
        assert_eq!(timestamp("2024-01-10 10:00:00.470432+00:00"), "2024-01-10T10:00:00Z");
        assert_eq!(timestamp(""), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn maps_languages() {
        assert_eq!(iso639_1("ita"), "it");
        assert_eq!(iso639_1("xyz"), "en");
    }
}
