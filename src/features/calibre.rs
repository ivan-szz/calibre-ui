//! Changes to the library go through Calibre's own command-line tools (`calibredb` and
//! `ebook-convert`), so the library stays exactly as Calibre expects it. Reads go
//! straight to `metadata.db` in [`library`](super::library).
//!
//! The tools are found on `PATH`, in the macOS app bundle, or at the paths in the
//! `CALIBREDB` and `EBOOK_CONVERT` environment variables. `calibredb` cannot write while
//! the Calibre desktop app has the same library open, unless the app's content server
//! is running.

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    sync::Mutex,
};

use tokio::process::Command;
use topcoat::{Error, Result};

use super::library::root;

/// Formats `ebook-convert` can produce, offered for conversion.
pub const OUTPUT_FORMATS: [&str; 7] = ["EPUB", "AZW3", "MOBI", "PDF", "DOCX", "TXT", "KEPUB"];

fn tool(env: &str, name: &str) -> PathBuf {
    if let Some(path) = std::env::var_os(env).filter(|p| !p.is_empty()) {
        return path.into();
    }
    let bundled = Path::new("/Applications/calibre.app/Contents/MacOS").join(name);
    if bundled.exists() { bundled } else { name.into() }
}

async fn run(program: PathBuf, args: Vec<OsString>) -> Result<String> {
    let output = Command::new(&program).args(&args).output().await.map_err(|e| {
        Error::msg(format!("could not run {}: {e}. Is Calibre installed?", program.display()))
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::msg(format!("{} failed: {}", program.display(), stderr.trim())));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Runs a `calibredb` command on the library.
async fn calibredb(command: &str, args: impl IntoIterator<Item = OsString>) -> Result<String> {
    let mut all: Vec<OsString> = vec![command.into(), "--with-library".into(), root().into()];
    all.extend(args);
    run(tool("CALIBREDB", "calibredb"), all).await
}

/// Adds a book file to the library and returns the new book's id, or `None` when the
/// library already has the book. `calibredb add` then succeeds without adding it, and
/// says why only on stderr.
pub async fn add_book(file: &Path) -> Result<Option<i64>> {
    let out = calibredb("add", [file.into()]).await?;
    Ok(parse_added_id(&out))
}

/// Reads the id from `calibredb add` output such as `Added book ids: 12`.
fn parse_added_id(out: &str) -> Option<i64> {
    let ids = out.lines().find_map(|l| l.trim().strip_prefix("Added book ids:"))?;
    ids.split(',').next()?.trim().parse().ok()
}

/// A `YYYY-MM-DD` date as noon UTC. Calibre shifts a bare date by the server's
/// timezone (2024-01-01 became 2024-01-02 at UTC+1); noon UTC keeps the day everywhere.
pub fn calibre_date(value: &str) -> String {
    let is_date = value.len() == 10 && value.bytes().enumerate().all(|(i, b)| if i == 4 || i == 7 { b == b'-' } else { b.is_ascii_digit() });
    if is_date { format!("{value}T12:00:00+00:00") } else { value.to_owned() }
}

/// Sets metadata fields, as `(field, value)` pairs like `("title", "Dune")`. A
/// `rating` is in stars, 0-5, as `calibredb set_metadata` takes it.
pub async fn set_metadata(id: i64, fields: &[(&str, String)]) -> Result<()> {
    let mut args: Vec<OsString> = vec![id.to_string().into()];
    for (field, value) in fields {
        let value = if *field == "pubdate" { calibre_date(value) } else { value.clone() };
        args.push("--field".into());
        args.push(format!("{field}:{value}").into());
    }
    calibredb("set_metadata", args).await.map(drop)
}

/// Sets a custom column, by its lookup name without the `#`. The value is in
/// Calibre's stored form: ratings in half-stars, 0-10, unlike `set_metadata`.
pub async fn set_custom(id: i64, label: &str, value: &str) -> Result<()> {
    calibredb("set_custom", [label.into(), id.to_string().into(), value.into()]).await.map(drop)
}

pub async fn set_cover(id: i64, image: &Path) -> Result<()> {
    let mut field = OsString::from("cover:");
    field.push(image);
    calibredb("set_metadata", [id.to_string().into(), "--field".into(), field]).await.map(drop)
}

/// Removes a book. Calibre keeps it in the library's trash for a while.
pub async fn remove_book(id: i64) -> Result<()> {
    calibredb("remove", [id.to_string().into()]).await.map(drop)
}

pub async fn add_format(id: i64, file: &Path) -> Result<()> {
    calibredb("add_format", [id.to_string().into(), file.into()]).await.map(drop)
}

pub async fn remove_format(id: i64, format: &str) -> Result<()> {
    calibredb("remove_format", [id.to_string().into(), format.into()]).await.map(drop)
}

/// Metadata found online by `fetch-ebook-metadata`, as `calibredb set_metadata`
/// fields: `(field, value)` pairs like `("title", "Dune")`, plus the ISBN to find the
/// cover again.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fetched {
    pub fields: Vec<(&'static str, String)>,
    pub isbn: Option<String>,
}

/// Looks a book up in Calibre's online metadata sources. `None` when nothing matches.
pub async fn fetch_metadata(title: &str, authors: &str, isbn: Option<&str>) -> Result<Option<Fetched>> {
    // Calibre waits up to 30 s per source by default, which kept the page loading for
    // about two minutes in testing.
    let mut args: Vec<OsString> =
        vec!["--title".into(), title.into(), "--authors".into(), authors.into(), "--opf".into(), "--timeout".into(), "15".into()];
    if let Some(isbn) = isbn {
        args.extend(["--isbn".into(), isbn.into()]);
    }
    let program = tool("FETCH_EBOOK_METADATA", "fetch-ebook-metadata");
    let output = Command::new(&program).args(&args).output().await.map_err(|e| {
        Error::msg(format!("could not run {}: {e}. Is Calibre installed?", program.display()))
    })?;
    let opf = String::from_utf8_lossy(&output.stdout);
    // It exits with an error when no source has the book.
    if !output.status.success() || !opf.contains("<package") {
        return Ok(None);
    }
    Ok(Some(parse_opf(&opf)))
}

/// Saves the cover Calibre's sources have for this ISBN to `path`. Returns whether
/// one was found.
pub async fn fetch_cover(title: &str, authors: &str, isbn: Option<&str>, path: &Path) -> Result<bool> {
    let mut args: Vec<OsString> =
        vec!["--title".into(), title.into(), "--authors".into(), authors.into(), "--cover".into(), path.into()];
    if let Some(isbn) = isbn {
        args.extend(["--isbn".into(), isbn.into()]);
    }
    let _ = run(tool("FETCH_EBOOK_METADATA", "fetch-ebook-metadata"), args).await;
    Ok(path.metadata().is_ok_and(|m| m.len() > 0))
}

fn unescape_xml(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

/// Every `<tag ...>content</tag>` in `xml`, as `(attributes, unescaped content)`.
fn elements<'a>(xml: &'a str, tag: &str) -> Vec<(&'a str, String)> {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let mut found = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find(&open) {
        let after = &rest[start + open.len()..];
        // Skip longer tag names that share the prefix, like `dc:creatorX`.
        if !after.starts_with([' ', '>', '\n', '\t', '/']) {
            rest = after;
            continue;
        }
        let Some(tag_end) = after.find('>') else { break };
        let attrs = &after[..tag_end];
        if attrs.ends_with('/') {
            found.push((attrs, String::new()));
            rest = &after[tag_end..];
            continue;
        }
        let body = &after[tag_end + 1..];
        let Some(end) = body.find(&close) else { break };
        found.push((attrs, unescape_xml(body[..end].trim())));
        rest = &body[end..];
    }
    found
}

/// The value of attribute `name` in a tag's attribute text.
fn attr<'a>(attrs: &'a str, name: &str) -> Option<&'a str> {
    let start = attrs.find(&format!("{name}=\""))? + name.len() + 2;
    Some(&attrs[start..start + attrs[start..].find('"')?])
}

/// The `content` of `<meta name="...">` in an OPF.
fn opf_meta(xml: &str, name: &str) -> Option<String> {
    elements(xml, "meta")
        .into_iter()
        .find(|(attrs, _)| attr(attrs, "name") == Some(name))
        .and_then(|(attrs, _)| attr(attrs, "content").map(unescape_xml))
}

fn parse_opf(xml: &str) -> Fetched {
    let texts = |tag: &str| -> Vec<String> {
        elements(xml, tag).into_iter().map(|(_, text)| text).filter(|t| !t.is_empty()).collect()
    };
    let mut fields = Vec::new();
    let mut push = |field: &'static str, value: String| {
        if !value.is_empty() {
            fields.push((field, value));
        }
    };
    push("title", texts("dc:title").into_iter().next().unwrap_or_default());
    push("authors", texts("dc:creator").join(" & "));
    push("publisher", texts("dc:publisher").into_iter().next().unwrap_or_default());
    // Calibre writes 0101-01-01 for an unknown date.
    push(
        "pubdate",
        texts("dc:date").into_iter().next().filter(|d| !d.starts_with("0101")).map(|d| d.chars().take(10).collect()).unwrap_or_default(),
    );
    push("tags", texts("dc:subject").join(", "));
    push("languages", texts("dc:language").join(", "));
    let series = opf_meta(xml, "calibre:series").unwrap_or_default();
    if !series.is_empty() {
        push("series", series);
        push("series_index", opf_meta(xml, "calibre:series_index").unwrap_or_default());
    }
    // OPF ratings are 0-10; `calibredb` takes stars.
    if let Some(rating) = opf_meta(xml, "calibre:rating").and_then(|r| r.parse::<f64>().ok()) {
        push("rating", ((rating / 2.0).round() as u32).to_string());
    }
    push("comments", texts("dc:description").into_iter().next().unwrap_or_default());

    let isbn = elements(xml, "dc:identifier")
        .into_iter()
        .find(|(attrs, text)| attrs.to_ascii_lowercase().contains("isbn") || text.to_ascii_lowercase().starts_with("isbn:"))
        .map(|(_, text)| text.trim_start_matches("isbn:").trim_start_matches("ISBN:").to_owned());
    Fetched { fields, isbn }
}

/// A fresh, empty temporary directory.
pub fn temp_dir() -> Result<PathBuf> {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    let dir = std::env::temp_dir().join(format!("calibre-ui-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Converts a book file to another format with `ebook-convert` and adds the result to
/// the book.
pub async fn convert(id: i64, source: &Path, to: &str) -> Result<()> {
    let dir = temp_dir()?;
    let target = dir.join(format!("book.{}", to.to_lowercase()));
    let result = async {
        run(tool("EBOOK_CONVERT", "ebook-convert"), vec![source.into(), target.clone().into()]).await?;
        add_format(id, &target).await
    }
    .await;
    let _ = std::fs::remove_dir_all(dir);
    result
}

/// A background job, such as a conversion, shown on the tasks page.
#[derive(Clone, Debug)]
pub struct Task {
    pub label: String,
    pub user_id: u64,
    pub status: TaskStatus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskStatus {
    Running,
    Done,
    Failed(String),
}

// shortcut: tasks live in memory and are lost on restart; persist them if that matters.
static TASKS: Mutex<Vec<Task>> = Mutex::new(Vec::new());

/// Runs `job` in the background and records its outcome under `label`.
pub fn spawn_task(label: String, user_id: u64, job: impl Future<Output = Result<()>> + Send + 'static) {
    let index = {
        let mut tasks = TASKS.lock().unwrap();
        tasks.push(Task { label, user_id, status: TaskStatus::Running });
        tasks.len() - 1
    };
    tokio::spawn(async move {
        let status = match job.await {
            Ok(()) => TaskStatus::Done,
            Err(error) => TaskStatus::Failed(format!("{error:#}")),
        };
        TASKS.lock().unwrap()[index].status = status;
    });
}

/// Every task, newest first.
pub fn tasks() -> Vec<Task> {
    TASKS.lock().unwrap().iter().rev().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fetched_opf() {
        let opf = r#"<?xml version='1.0' encoding='utf-8'?>
<package xmlns="http://www.idpf.org/2007/opf" unique-identifier="uuid_id" version="2.0">
    <metadata xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:opf="http://www.idpf.org/2007/opf">
        <dc:identifier opf:scheme="calibre" id="calibre_id">1</dc:identifier>
        <dc:identifier opf:scheme="ISBN">9780441013593</dc:identifier>
        <dc:title>Dune</dc:title>
        <dc:creator opf:file-as="Herbert, Frank" opf:role="aut">Frank Herbert</dc:creator>
        <dc:date>1965-08-01T00:00:00+00:00</dc:date>
        <dc:description>&lt;p&gt;Spice &amp;amp; sand&lt;/p&gt;</dc:description>
        <dc:publisher>Ace</dc:publisher>
        <dc:language>eng</dc:language>
        <dc:subject>Science Fiction</dc:subject>
        <dc:subject>Classics</dc:subject>
        <meta name="calibre:series" content="Dune"/>
        <meta name="calibre:series_index" content="1.0"/>
        <meta name="calibre:rating" content="8.0"/>
    </metadata>
</package>"#;
        let fetched = parse_opf(opf);
        assert_eq!(fetched.isbn.as_deref(), Some("9780441013593"));
        assert_eq!(
            fetched.fields,
            [
                ("title", "Dune".to_owned()),
                ("authors", "Frank Herbert".to_owned()),
                ("publisher", "Ace".to_owned()),
                ("pubdate", "1965-08-01".to_owned()),
                ("tags", "Science Fiction, Classics".to_owned()),
                ("languages", "eng".to_owned()),
                ("series", "Dune".to_owned()),
                ("series_index", "1.0".to_owned()),
                ("rating", "4".to_owned()),
                ("comments", "<p>Spice &amp; sand</p>".to_owned()),
            ]
        );
    }

    #[test]
    fn pins_bare_dates_to_noon_utc() {
        assert_eq!(calibre_date("1957-06-01"), "1957-06-01T12:00:00+00:00");
        assert_eq!(calibre_date("1957"), "1957");
        assert_eq!(calibre_date("1957-06-01T00:00:00Z"), "1957-06-01T00:00:00Z");
    }

    #[test]
    fn parses_added_ids() {
        assert_eq!(parse_added_id("Backing up metadata\nAdded book ids: 12\n"), Some(12));
        assert_eq!(parse_added_id("Added book ids: 3, 4"), Some(3));
        assert_eq!(parse_added_id("The following books were not added as they already exist"), None);
    }
}
