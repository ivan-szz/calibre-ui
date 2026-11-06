//! An OPDS 1.2 catalog, so e-reader apps (KOReader, Moon+ Reader, Thorium...) can
//! browse and download the library. Apps sign in with HTTP Basic credentials.

pub mod category;

use topcoat::{
    Result,
    context::{Cx, memoize},
    router::{header, href, query_params, route},
};

use super::{
    books::book_id::{BookId, cover::cover, download::format::{Format, download}},
    ui::{ListQuery, encode},
};
use crate::{app::kind_label, t};
use crate::features::{
    library::{self, BookSummary, Filter, Kind, PAGE_SIZE, content_type},
};
use category::Category;

const NAVIGATION: &str = "application/atom+xml;profile=opds-catalog;kind=navigation";
const ACQUISITION: &str = "application/atom+xml;profile=opds-catalog;kind=acquisition";

type Feed = ([(header::HeaderName, &'static str); 1], String);

/// Escapes text for XML content and attributes.
pub fn xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Calibre's `2024-01-10 10:00:00+00:00` as an Atom date.
fn atom_date(stored: &str) -> String {
    match stored.get(..19) {
        Some(date) => format!("{}Z", date.replacen(' ', "T", 1)),
        None => "1970-01-01T00:00:00Z".to_owned(),
    }
}

/// A feed document around `entries`.
fn feed(cx: &Cx, id: &str, title: &str, kind: &'static str, links: &str, entries: &str) -> Feed {
    let start = href!(catalog).resolve(cx);
    let search_url = href!(search).resolve(cx);
    (
        [(header::CONTENT_TYPE, kind)],
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom" xmlns:opds="http://opds-spec.org/2010/catalog" xmlns:dc="http://purl.org/dc/terms/">
<id>urn:calibre-ui:{id}</id>
<title>{}</title>
<updated>{}</updated>
<link rel="start" href="{start}" type="{NAVIGATION}"/>
<link rel="search" href="{search_url}?q={{searchTerms}}" type="{ACQUISITION}"/>
{links}{entries}</feed>
"#,
            xml(title),
            updated(cx),
        ),
    )
}

/// When the library last changed, as the `updated` date of feeds and their links.
#[memoize]
fn updated(cx: &Cx) -> String {
    atom_date(&library::last_modified().unwrap_or_default())
}

/// A navigation entry linking to another feed.
pub fn nav_entry(cx: &Cx, id: &str, title: &str, content: &str, href: &str, kind: &str) -> String {
    format!(
        "<entry><title>{}</title><id>urn:calibre-ui:{id}</id><updated>{}</updated><content type=\"text\">{}</content><link rel=\"subsection\" href=\"{}\" type=\"{kind}\"/></entry>\n",
        xml(title),
        updated(cx),
        xml(content),
        xml(href),
    )
}

/// One page of books as an acquisition feed, with a next link while more remain.
async fn book_feed(cx: &Cx, id: &str, title: &str, filter: Filter<'_>, sort: &str) -> Result<Feed> {
    let query = query_params::<ListQuery>(cx)?;
    let page = query.page.unwrap_or(1).max(1);
    let (books, total) = library::books(&filter, sort, page, &super::hidden_tags(cx).await)?;
    let mut links = String::new();
    if page * PAGE_SIZE < total {
        let path = topcoat::router::request::uri(cx).path().to_owned();
        let q = query.q.as_deref().map(|q| format!("&q={}", encode(q))).unwrap_or_default();
        links = format!("<link rel=\"next\" href=\"{}?page={}{}\" type=\"{ACQUISITION}\"/>\n", xml(&path), page + 1, xml(&q));
    }
    let entries: String = books.iter().map(|b| book_entry(cx, b)).collect();
    Ok(feed(cx, id, title, ACQUISITION, &links, &entries))
}

fn book_entry(cx: &Cx, book: &BookSummary) -> String {
    let mut links = String::new();
    if book.has_cover {
        let cover_url = href!(cover, BookId(book.id)).resolve(cx);
        links += &format!(
            "<link rel=\"http://opds-spec.org/image\" href=\"{cover_url}\" type=\"image/jpeg\"/><link rel=\"http://opds-spec.org/image/thumbnail\" href=\"{cover_url}\" type=\"image/jpeg\"/>"
        );
    }
    for format in &book.formats {
        let file = href!(download, BookId(book.id), Format(format.to_lowercase())).resolve(cx);
        links += &format!(
            "<link rel=\"http://opds-spec.org/acquisition\" href=\"{}\" type=\"{}\" title=\"{}\"/>",
            xml(&file),
            content_type(format),
            xml(format),
        );
    }
    let authors: String = book
        .authors
        .split(" & ")
        .filter(|a| !a.is_empty())
        .map(|a| format!("<author><name>{}</name></author>", xml(a)))
        .collect();
    format!(
        "<entry><title>{}</title><id>urn:calibre-ui:book:{}</id><updated>{}</updated>{authors}{links}</entry>\n",
        xml(&book.title),
        book.id,
        atom_date(&book.last_modified),
    )
}

/// The catalog root.
#[route(GET)]
pub async fn catalog(cx: &Cx) -> Result<Feed> {
    let mut entries = nav_entry(cx, "new", &t!(opds_newest).resolve(cx), &t!(opds_newest_hint).resolve(cx), &href!(newest).resolve(cx), ACQUISITION);
    entries += &nav_entry(cx, "discover", &t!(nav_discover).resolve(cx), &t!(discover_subtitle).resolve(cx), &href!(discover).resolve(cx), ACQUISITION);
    for kind in [Kind::Authors, Kind::Series, Kind::Tags, Kind::Publishers, Kind::Languages] {
        let link = href!(category::list, Category(kind)).resolve(cx);
        let label = kind_label(kind).resolve(cx);
        entries += &nav_entry(cx, &kind.to_string(), &label, &label, &link, NAVIGATION);
    }
    Ok(feed(cx, "root", &t!(app_name).resolve(cx), NAVIGATION, "", &entries))
}

#[route(GET "./new")]
pub async fn newest(cx: &Cx) -> Result<Feed> {
    book_feed(cx, "new", &t!(opds_newest).resolve(cx), Filter::All, "new").await
}

#[route(GET "./discover")]
pub async fn discover(cx: &Cx) -> Result<Feed> {
    let (books, _) = library::books(&Filter::All, "random", 1, &super::hidden_tags(cx).await)?;
    let entries: String = books.iter().map(|b| book_entry(cx, b)).collect();
    Ok(feed(cx, "discover", &t!(nav_discover).resolve(cx), ACQUISITION, "", &entries))
}

#[route(GET "./search")]
pub async fn search(cx: &Cx) -> Result<Feed> {
    let q = query_params::<ListQuery>(cx)?.q.clone().unwrap_or_default();
    let title = t!(search_results, query = q.as_str()).resolve(cx).into_owned();
    book_feed(cx, "search", &title, Filter::Search(&q), "title").await
}

/// The OpenSearch description, for apps and browsers that discover search this way.
#[route(GET "./osd")]
pub async fn osd(cx: &Cx) -> Result<([(header::HeaderName, &'static str); 1], String)> {
    let search_url = href!(search).resolve(cx);
    Ok((
        [(header::CONTENT_TYPE, "application/opensearchdescription+xml")],
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<OpenSearchDescription xmlns="http://a9.com/-/spec/opensearch/1.1/">
<ShortName>Library</ShortName><Description>Search the library</Description>
<Url type="{ACQUISITION}" template="{search_url}?q={{searchTerms}}"/>
</OpenSearchDescription>
"#
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_xml_and_dates() {
        assert_eq!(xml("Tom & \"Jerry\" <3"), "Tom &amp; &quot;Jerry&quot; &lt;3");
        assert_eq!(atom_date("2024-01-10 10:00:00+00:00"), "2024-01-10T10:00:00Z");
        assert_eq!(atom_date(""), "1970-01-01T00:00:00Z");
    }
}
