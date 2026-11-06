//! Read-only access to a Calibre library: its `metadata.db` and the book files beside it.
//!
//! The library folder comes from the `CALIBRE_LIBRARY` environment variable and defaults
//! to `./library`.

use std::{fmt, path::PathBuf, str::FromStr};

use rusqlite::{Connection, OpenFlags, OptionalExtension, params, params_from_iter, types::Value};
use topcoat::Result;

/// Books shown on one page of a list.
pub const PAGE_SIZE: i64 = 30;

/// The sort orders a book list offers, by key.
pub const SORTS: [&str; 5] = ["new", "old", "title", "author", "pubdate"];

/// The library folder, which holds `metadata.db` and one folder per book.
pub fn root() -> PathBuf {
    std::env::var_os("CALIBRE_LIBRARY").map_or_else(|| "library".into(), PathBuf::from)
}

// shortcut: one blocking connection per call on the async thread; fine for a local
// SQLite file, move to a pool and spawn_blocking if the library is large or busy.
fn open() -> Result<Connection> {
    Ok(Connection::open_with_flags(
        root().join("metadata.db"),
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?)
}

/// A kind of category that books can be browsed by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Authors,
    Series,
    Tags,
    Publishers,
    Languages,
    Ratings,
    Formats,
}

impl Kind {
    pub const ALL: [Kind; 7] = [
        Kind::Authors,
        Kind::Series,
        Kind::Tags,
        Kind::Publishers,
        Kind::Languages,
        Kind::Ratings,
        Kind::Formats,
    ];

    fn slug(self) -> &'static str {
        match self {
            Kind::Authors => "authors",
            Kind::Series => "series",
            Kind::Tags => "tags",
            Kind::Publishers => "publishers",
            Kind::Languages => "languages",
            Kind::Ratings => "ratings",
            Kind::Formats => "formats",
        }
    }

    /// `(table, name column, sort column, link table, link column)`. Formats live in
    /// the `data` table and have no table of their own, so they are handled apart.
    fn tables(self) -> (&'static str, &'static str, &'static str, &'static str, &'static str) {
        match self {
            Kind::Authors => ("authors", "name", "sort", "books_authors_link", "author"),
            Kind::Series => ("series", "name", "sort", "books_series_link", "series"),
            Kind::Tags => ("tags", "name", "name", "books_tags_link", "tag"),
            Kind::Publishers => ("publishers", "name", "sort", "books_publishers_link", "publisher"),
            Kind::Languages => ("languages", "lang_code", "lang_code", "books_languages_link", "lang_code"),
            Kind::Ratings => ("ratings", "rating", "rating DESC", "books_ratings_link", "rating"),
            Kind::Formats => ("data", "format", "format", "data", "book"),
        }
    }

    /// How an item's stored name is shown: ratings as stars, language codes in capitals.
    pub fn display(self, name: &str) -> String {
        match self {
            Kind::Ratings => "★".repeat(name.parse::<usize>().unwrap_or(0) / 2),
            Kind::Languages => name.to_uppercase(),
            _ => name.to_owned(),
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.slug())
    }
}

impl FromStr for Kind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Kind::ALL
            .into_iter()
            .find(|kind| kind.slug() == s)
            .ok_or_else(|| format!("unknown category `{s}`"))
    }
}

/// One author, tag, series... Ids are text so formats, keyed by name, fit too.
#[derive(Clone, Debug)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub count: i64,
}

/// A condition keeping the book ids in `col` that carry none of the `hidden` tags, the
/// tags an admin hid from a user. Tag names compare case-insensitively.
fn not_hidden(col: &str, hidden: &[String]) -> (String, Vec<Value>) {
    if hidden.is_empty() {
        return ("1".into(), vec![]);
    }
    (
        format!(
            "{col} NOT IN (SELECT l.book FROM books_tags_link l JOIN tags t ON t.id = l.tag \
             WHERE lower(t.name) IN ({}))",
            vec!["?"; hidden.len()].join(",")
        ),
        hidden.iter().map(|t| Value::Text(t.to_lowercase())).collect(),
    )
}

/// Whether the book exists and the user may see it.
fn visible(conn: &Connection, id: i64, hidden: &[String]) -> Result<bool> {
    let (cond, mut params) = not_hidden("id", hidden);
    params.insert(0, Value::Integer(id));
    let found = conn
        .query_row(&format!("SELECT 1 FROM books WHERE id = ? AND {cond}"), params_from_iter(&params), |_| Ok(()))
        .optional()?;
    Ok(found.is_some())
}

/// Every item of `kind` that has at least one visible book, with its book count.
pub fn items(kind: Kind, hidden: &[String]) -> Result<Vec<Item>> {
    let sql = match kind {
        Kind::Formats => {
            let (cond, _) = not_hidden("book", hidden);
            format!("SELECT format, format, count(DISTINCT book) FROM data WHERE {cond} GROUP BY format ORDER BY format")
        }
        _ => {
            let (table, name, sort, link, col) = kind.tables();
            let (cond, _) = not_hidden("l.book", hidden);
            format!(
                "SELECT CAST(t.id AS TEXT), CAST(t.{name} AS TEXT), count(*) FROM {table} t \
                 JOIN {link} l ON l.{col} = t.id WHERE {cond} GROUP BY t.id ORDER BY t.{sort}"
            )
        }
    };
    let (_, params) = not_hidden("", hidden);
    let conn = open()?;
    let mut stmt = conn.prepare(&sql)?;
    let items = stmt
        .query_map(params_from_iter(&params), |r| Ok(Item { id: r.get(0)?, name: r.get(1)?, count: r.get(2)? }))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(items)
}

/// The stored name of one item, or `None` if it does not exist.
pub fn item_name(kind: Kind, id: &str) -> Result<Option<String>> {
    let conn = open()?;
    let name = match kind {
        Kind::Formats => conn
            .query_row("SELECT format FROM data WHERE format = ? LIMIT 1", [id], |r| r.get(0))
            .optional()?,
        _ => {
            let (table, name, ..) = kind.tables();
            conn.query_row(
                &format!("SELECT CAST({name} AS TEXT) FROM {table} WHERE id = ?"),
                [id],
                |r| r.get(0),
            )
            .optional()?
        }
    };
    Ok(name)
}

/// Which books a list shows.
pub enum Filter<'a> {
    All,
    /// Title, author, tag, or series containing the text.
    Search(&'a str),
    /// Books linked to one category item.
    Item(Kind, &'a str),
    /// Books with these ids, such as the books on a shelf.
    Ids(&'a [u64]),
    /// Books without these ids, such as the unread ones.
    NotIds(&'a [u64]),
}

impl Filter<'_> {
    /// A `WHERE` condition on `books b` and its parameters.
    fn sql(&self) -> (String, Vec<Value>) {
        match *self {
            Filter::All => ("1".into(), vec![]),
            Filter::Search(text) => (
                "(b.title LIKE ?1 \
                 OR b.id IN (SELECT l.book FROM books_authors_link l JOIN authors a ON a.id = l.author WHERE a.name LIKE ?1) \
                 OR b.id IN (SELECT l.book FROM books_tags_link l JOIN tags t ON t.id = l.tag WHERE t.name LIKE ?1) \
                 OR b.id IN (SELECT l.book FROM books_series_link l JOIN series s ON s.id = l.series WHERE s.name LIKE ?1))"
                    .into(),
                vec![Value::Text(format!("%{}%", text.trim()))],
            ),
            Filter::Item(Kind::Formats, id) => {
                ("b.id IN (SELECT book FROM data WHERE format = ?)".into(), vec![Value::Text(id.into())])
            }
            Filter::Item(kind, id) => {
                let (.., link, col) = kind.tables();
                (format!("b.id IN (SELECT book FROM {link} WHERE {col} = ?)"), vec![Value::Text(id.into())])
            }
            Filter::Ids(ids) | Filter::NotIds(ids) => (
                format!(
                    "b.id {} ({})",
                    if matches!(self, Filter::Ids(_)) { "IN" } else { "NOT IN" },
                    // `IN ()` is a syntax error. An empty list includes nothing (`IN (NULL)`)
                    // or excludes nothing (`NOT IN (0)`: book ids start at 1).
                    if ids.is_empty() {
                        if matches!(self, Filter::Ids(_)) { "NULL".to_owned() } else { "0".to_owned() }
                    } else {
                        vec!["?"; ids.len()].join(",")
                    },
                ),
                ids.iter().map(|&id| Value::Integer(id as i64)).collect(),
            ),
        }
    }
}

/// A book as shown in a grid.
#[derive(Clone, Debug)]
pub struct BookSummary {
    pub id: i64,
    pub title: String,
    pub authors: String,
    pub has_cover: bool,
    pub series: Option<String>,
    pub series_index: f64,
    /// Format names such as `EPUB`, for the OPDS feed.
    pub formats: Vec<String>,
    /// When Calibre last changed the book, as stored (`YYYY-MM-DD hh:mm:ss+00:00`).
    pub last_modified: String,
}

/// The `ORDER BY` clause for a sort key from [`SORTS`], plus `series` and `random`.
fn order_by(sort: &str) -> &'static str {
    match sort {
        "old" => "b.timestamp ASC",
        "title" => "b.sort ASC",
        "author" => "b.author_sort ASC, b.sort ASC",
        "pubdate" => "b.pubdate DESC",
        "series" => "b.series_index ASC",
        "random" => "random()",
        _ => "b.timestamp DESC",
    }
}

/// One page (from 1) of the visible books matching `filter`, and how many match in total.
pub fn books(filter: &Filter, sort: &str, page: i64, hidden: &[String]) -> Result<(Vec<BookSummary>, i64)> {
    let conn = open()?;
    let (cond, mut params) = filter.sql();
    let (visible, hidden_params) = not_hidden("b.id", hidden);
    let cond = format!("({cond}) AND {visible}");
    params.extend(hidden_params);
    let total = conn.query_row(
        &format!("SELECT count(*) FROM books b WHERE {cond}"),
        params_from_iter(&params),
        |r| r.get(0),
    )?;
    let sql = format!(
        "SELECT b.id, b.title, b.has_cover, \
         (SELECT group_concat(a.name, ' & ') FROM books_authors_link l JOIN authors a ON a.id = l.author WHERE l.book = b.id), \
         (SELECT s.name FROM books_series_link l JOIN series s ON s.id = l.series WHERE l.book = b.id), \
         b.series_index, \
         (SELECT group_concat(format) FROM data WHERE book = b.id), \
         coalesce(b.last_modified, b.timestamp, '') \
         FROM books b WHERE {cond} ORDER BY {} LIMIT {PAGE_SIZE} OFFSET {}",
        order_by(sort),
        (page.max(1) - 1) * PAGE_SIZE,
    );
    let mut stmt = conn.prepare(&sql)?;
    let books = stmt
        .query_map(params_from_iter(&params), |r| {
            Ok(BookSummary {
                id: r.get(0)?,
                title: r.get(1)?,
                has_cover: r.get(2)?,
                authors: r.get::<_, Option<String>>(3)?.unwrap_or_default(),
                series: r.get(4)?,
                series_index: r.get(5)?,
                formats: r
                    .get::<_, Option<String>>(6)?
                    .map(|f| f.split(',').map(str::to_owned).collect())
                    .unwrap_or_default(),
                last_modified: r.get(7)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok((books, total))
}

/// A downloadable file of a book.
#[derive(Clone, Debug)]
pub struct Format {
    pub format: String,
    pub size: i64,
}

/// Everything the book page shows.
#[derive(Clone, Debug)]
pub struct Book {
    pub id: i64,
    /// Calibre's stable id for the book, which Kobo devices use.
    pub uuid: String,
    /// When the book was added and last changed, as stored.
    pub added: String,
    pub last_modified: String,
    /// The publication date as stored, empty when unknown.
    pub pubdate: String,
    pub title: String,
    pub has_cover: bool,
    /// The publication date as `YYYY-MM-DD`, if known.
    pub published: Option<String>,
    pub series_index: f64,
    pub authors: Vec<Item>,
    pub series: Vec<Item>,
    pub tags: Vec<Item>,
    pub publishers: Vec<Item>,
    pub languages: Vec<Item>,
    pub ratings: Vec<Item>,
    /// The description as plain-text paragraphs.
    pub description: Vec<String>,
    pub formats: Vec<Format>,
    /// `(type, value)` pairs such as `("isbn", "978...")`.
    pub identifiers: Vec<(String, String)>,
    /// The library's own columns, created in Calibre.
    pub custom: Vec<CustomField>,
}

/// A book's value in one of the library's custom columns.
#[derive(Clone, Debug)]
pub struct CustomField {
    /// The lookup name `calibredb set_custom` takes, without the `#`.
    pub label: String,
    /// The heading shown in Calibre.
    pub name: String,
    pub datatype: String,
    pub is_multiple: bool,
    /// The value as text: lists joined with commas, ratings as stars 0-5 in digits,
    /// yes/no as `Yes` or `No`, dates as `YYYY-MM-DD`. Empty when unset.
    pub value: String,
}

impl CustomField {
    /// Whether the column holds a value Calibre computes (`composite`) and cannot be set.
    pub fn editable(&self) -> bool {
        self.datatype != "composite"
    }

    /// The value as shown to readers.
    pub fn display(&self) -> String {
        match self.datatype.as_str() {
            "rating" => "★".repeat(self.value.parse().unwrap_or(0)),
            _ => self.value.clone(),
        }
    }
}

/// The book's values in every custom column, in the order Calibre created them.
fn custom_fields(conn: &Connection, book: i64) -> Result<Vec<CustomField>> {
    // Old libraries may predate custom columns.
    let exists = conn
        .query_row("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'custom_columns'", [], |_| Ok(()))
        .optional()?;
    if exists.is_none() {
        return Ok(vec![]);
    }
    let columns: Vec<(i64, String, String, String, bool, bool)> = conn
        .prepare("SELECT id, label, name, datatype, is_multiple, normalized FROM custom_columns WHERE NOT mark_for_delete ORDER BY id")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)))?
        .collect::<rusqlite::Result<_>>()?;

    let mut fields = Vec::new();
    for (id, label, name, datatype, is_multiple, normalized) in columns {
        let values: Vec<String> = match datatype.as_str() {
            // Calibre computes these from a template; there is nothing stored to read.
            "composite" => vec![],
            _ if normalized => conn
                .prepare(&format!(
                    "SELECT CAST(c.value AS TEXT) FROM custom_column_{id} c \
                     JOIN books_custom_column_{id}_link l ON l.value = c.id WHERE l.book = ? ORDER BY l.id"
                ))?
                .query_map([book], |r| r.get(0))?
                .collect::<rusqlite::Result<_>>()?,
            _ => conn
                .prepare(&format!("SELECT CAST(value AS TEXT) FROM custom_column_{id} WHERE book = ?"))?
                .query_map([book], |r| r.get(0))?
                .collect::<rusqlite::Result<_>>()?,
        };
        let value = match datatype.as_str() {
            "bool" => values.first().map_or(String::new(), |v| if v == "1" { "Yes".into() } else { "No".into() }),
            // Stored as half-stars 0-10.
            "rating" => values.first().map_or(String::new(), |v| (v.parse::<u32>().unwrap_or(0) / 2).to_string()),
            "datetime" => values.first().map_or(String::new(), |v| v.get(..10).unwrap_or(v).to_owned()),
            "comments" => values.first().map_or(String::new(), |v| strip_html(v).join("\n\n")),
            _ => values.join(", "),
        };
        fields.push(CustomField { label, name, datatype, is_multiple, value });
    }
    Ok(fields)
}

/// The items of `kind` linked to one book, in their stored order.
fn book_items(conn: &Connection, kind: Kind, book: i64) -> Result<Vec<Item>> {
    let (table, name, _, link, col) = kind.tables();
    let mut stmt = conn.prepare(&format!(
        "SELECT CAST(t.id AS TEXT), CAST(t.{name} AS TEXT) FROM {table} t \
         JOIN {link} l ON l.{col} = t.id WHERE l.book = ? ORDER BY l.id"
    ))?;
    let items = stmt
        .query_map([book], |r| Ok(Item { id: r.get(0)?, name: r.get(1)?, count: 0 }))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(items)
}

/// One book, or `None` if the library has no book with that id that the user may see.
pub fn book(id: i64, hidden: &[String]) -> Result<Option<Book>> {
    let conn = open()?;
    if !visible(&conn, id, hidden)? {
        return Ok(None);
    }
    let Some((title, has_cover, pubdate, series_index, uuid, added, last_modified)) = conn
        .query_row(
            "SELECT title, has_cover, pubdate, series_index, coalesce(uuid, ''), coalesce(timestamp, ''), \
             coalesce(last_modified, timestamp, '') FROM books WHERE id = ?",
            [id],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, String>(6)?,
                ))
            },
        )
        .optional()?
    else {
        return Ok(None);
    };
    // Calibre stores an unknown date as year 101.
    let pubdate = pubdate.filter(|d| d.len() >= 10 && !d.starts_with("0101"));
    let description = conn
        .query_row("SELECT text FROM comments WHERE book = ?", [id], |r| r.get::<_, String>(0))
        .optional()?
        .map(|html| strip_html(&html))
        .unwrap_or_default();
    let formats = conn
        .prepare("SELECT format, uncompressed_size FROM data WHERE book = ? ORDER BY format")?
        .query_map([id], |r| Ok(Format { format: r.get(0)?, size: r.get(1)? }))?
        .collect::<rusqlite::Result<_>>()?;
    let identifiers = conn
        .prepare("SELECT type, val FROM identifiers WHERE book = ? ORDER BY type")?
        .query_map([id], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;

    Ok(Some(Book {
        id,
        uuid,
        added,
        last_modified,
        published: pubdate.as_ref().map(|d| d[..10].to_owned()),
        pubdate: pubdate.unwrap_or_default(),
        title,
        has_cover,
        series_index,
        authors: book_items(&conn, Kind::Authors, id)?,
        series: book_items(&conn, Kind::Series, id)?,
        tags: book_items(&conn, Kind::Tags, id)?,
        publishers: book_items(&conn, Kind::Publishers, id)?,
        languages: book_items(&conn, Kind::Languages, id)?,
        ratings: book_items(&conn, Kind::Ratings, id)?,
        description,
        formats,
        identifiers,
        custom: custom_fields(&conn, id)?,
    }))
}

/// The id of the visible book with this Calibre uuid.
pub fn book_id_by_uuid(uuid: &str, hidden: &[String]) -> Result<Option<i64>> {
    let conn = open()?;
    let id: Option<i64> = conn.query_row("SELECT id FROM books WHERE uuid = ?", [uuid], |r| r.get(0)).optional()?;
    match id {
        Some(id) if visible(&conn, id, hidden)? => Ok(Some(id)),
        _ => Ok(None),
    }
}

/// Up to `limit` visible books with an EPUB or KEPUB changed after `since` (as
/// stored), oldest change first, as `(id, last_modified)`. Kobo devices read only these
/// formats.
pub fn kobo_changes(since: &str, limit: usize, hidden: &[String]) -> Result<Vec<(i64, String)>> {
    let (cond, mut params) = not_hidden("b.id", hidden);
    params.insert(0, Value::Text(since.to_owned()));
    let sql = format!(
        "SELECT b.id, coalesce(b.last_modified, b.timestamp, '') m FROM books b \
         WHERE m > ? AND {cond} AND b.id IN (SELECT book FROM data WHERE format IN ('EPUB', 'KEPUB')) \
         ORDER BY m, b.id LIMIT {limit}"
    );
    let conn = open()?;
    let rows = conn
        .prepare(&sql)?
        .query_map(params_from_iter(&params), |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}

/// When any book last changed, as stored.
pub fn last_modified() -> Result<String> {
    Ok(open()?.query_row("SELECT coalesce(max(last_modified), '') FROM books", [], |r| r.get(0))?)
}

/// The book's cover image, if it has one the user may see.
pub fn cover_file(id: i64, hidden: &[String]) -> Result<Option<PathBuf>> {
    let conn = open()?;
    if !visible(&conn, id, hidden)? {
        return Ok(None);
    }
    let path: Option<String> = conn
        .query_row("SELECT path FROM books WHERE id = ? AND has_cover", [id], |r| r.get(0))
        .optional()?;
    Ok(path.map(|path| root().join(path).join("cover.jpg")))
}

/// The file of one format of a book the user may see, with the name to download it as.
pub fn format_file(id: i64, format: &str, hidden: &[String]) -> Result<Option<(PathBuf, String)>> {
    let conn = open()?;
    if !visible(&conn, id, hidden)? {
        return Ok(None);
    }
    let row: Option<(String, String, String)> = conn
        .query_row(
            "SELECT b.path, d.name, d.format FROM data d JOIN books b ON b.id = d.book \
             WHERE d.book = ? AND d.format = ? COLLATE NOCASE",
            params![id, format],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    Ok(row.map(|(path, name, format)| {
        let file = format!("{name}.{}", format.to_lowercase());
        (root().join(path).join(&file), file)
    }))
}

/// The media type of an image in a comic archive, if the name is an image.
fn image_type(name: &str) -> Option<&'static str> {
    match name.rsplit_once('.')?.1.to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" => Some("image/jpeg"),
        "png" => Some("image/png"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

/// The image names of a CBZ comic, in reading order.
// shortcut: plain name order, which needs zero-padded page numbers (the norm); use a
// natural sort if comics with `1.jpg, 10.jpg, 2.jpg` show up.
fn comic_names(archive: &mut zip::ZipArchive<std::fs::File>) -> Vec<String> {
    let mut names: Vec<String> = archive
        .file_names()
        .filter(|n| !n.starts_with("__MACOSX/") && image_type(n).is_some())
        .map(str::to_owned)
        .collect();
    names.sort_by_key(|n| n.to_lowercase());
    names
}

/// How many pages a CBZ comic has.
pub fn comic_page_count(file: &std::path::Path) -> Result<usize> {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(file)?).map_err(|e| topcoat::Error::msg(e.to_string()))?;
    Ok(comic_names(&mut archive).len())
}

/// Page `index` (from 0) of a CBZ comic, with its media type.
pub fn comic_page(file: &std::path::Path, index: usize) -> Result<Option<(Vec<u8>, &'static str)>> {
    use std::io::Read as _;
    let mut archive = zip::ZipArchive::new(std::fs::File::open(file)?).map_err(|e| topcoat::Error::msg(e.to_string()))?;
    let Some(name) = comic_names(&mut archive).into_iter().nth(index) else {
        return Ok(None);
    };
    let mut entry = archive.by_name(&name).map_err(|e| topcoat::Error::msg(e.to_string()))?;
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes)?;
    Ok(image_type(&name).map(|kind| (bytes, kind)))
}

/// The media type of a book format such as `epub`.
pub fn content_type(format: &str) -> &'static str {
    match format.to_ascii_lowercase().as_str() {
        "epub" | "kepub" => "application/epub+zip",
        "pdf" => "application/pdf",
        "mobi" => "application/x-mobipocket-ebook",
        "azw3" => "application/vnd.amazon.ebook",
        "cbz" => "application/vnd.comicbook+zip",
        "cbr" => "application/vnd.comicbook-rar",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// A byte count as a short human-readable size.
pub fn human_size(bytes: i64) -> String {
    match bytes {
        b if b >= 1 << 20 => format!("{:.1} MB", b as f64 / (1 << 20) as f64),
        b if b >= 1 << 10 => format!("{} KB", b >> 10),
        b => format!("{b} B"),
    }
}

/// Turns Calibre's HTML description into plain-text paragraphs, so it can be shown
/// escaped. Script and style contents are dropped.
pub fn strip_html(html: &str) -> Vec<String> {
    let mut text = String::new();
    let mut tag: Option<String> = None;
    let mut skipping = false;
    for c in html.chars() {
        match (&mut tag, c) {
            (None, '<') => tag = Some(String::new()),
            (Some(t), '>') => {
                let closing = t.starts_with('/');
                let name = t
                    .trim_start_matches('/')
                    .split(|c: char| c.is_whitespace() || c == '/')
                    .next()
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if matches!(name.as_str(), "script" | "style") {
                    skipping = !closing;
                }
                if matches!(name.as_str(), "p" | "br" | "div" | "li" | "h1" | "h2" | "h3" | "h4") {
                    text.push('\n');
                }
                tag = None;
            }
            (Some(t), c) => t.push(c),
            (None, c) if !skipping => text.push(c),
            (None, _) => {}
        }
    }
    let text = text
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&amp;", "&");
    text.lines().map(str::trim).filter(|l| !l.is_empty()).map(str::to_owned).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_html_into_paragraphs() {
        assert_eq!(
            strip_html("<p>One &amp; <b>two</b></p><div>three<br/>four</div><script>alert(1)</script>"),
            ["One & two", "three", "four"]
        );
    }

    #[test]
    fn parses_kinds() {
        for kind in Kind::ALL {
            assert_eq!(kind.to_string().parse::<Kind>(), Ok(kind));
        }
        assert!("users".parse::<Kind>().is_err());
    }

    #[test]
    fn hides_tags_case_insensitively() {
        assert_eq!(not_hidden("b.id", &[]).0, "1");
        let (cond, params) = not_hidden("b.id", &["Adult".into(), "Spoilers".into()]);
        assert!(cond.starts_with("b.id NOT IN (") && cond.ends_with("IN (?,?))"));
        assert_eq!(params, [Value::Text("adult".into()), Value::Text("spoilers".into())]);
    }

    #[test]
    fn empty_id_filter_matches_nothing() {
        assert_eq!(Filter::Ids(&[]).sql().0, "b.id IN (NULL)");
        assert_eq!(Filter::NotIds(&[]).sql().0, "b.id NOT IN (0)");
        assert_eq!(Filter::NotIds(&[1, 2]).sql().0, "b.id NOT IN (?,?)");
    }
}
