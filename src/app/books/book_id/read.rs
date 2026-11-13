use serde::Deserialize;
use topcoat::{
    Result,
    asset::asset,
    context::Cx,
    font::{Font, fontsource::fontsource_font},
    icon::{icon, iconify::iconify_icon},
    router::{
        StatusCode,
        content::Json,
        error::{RouterErrorExt, bad_request},
        header, href, page, path_param, query_params, route,
    },
    view::{View, view},
};

use super::{BookId, download::format::{Format, download}, page as book_page};
use crate::{
    app::{hidden_tags, require},
    components::button::{ButtonSize, ButtonVariant, button_variants},
    features::{
        auth::Perm,
        library,
        reader::{FONTS, Prefs, ReadingPosition, THEMES},
    },
    t,
};

/// Formats the browser can show: EPUB through epub.js, PDF natively, CBZ page by page.
pub const READABLE: [&str; 3] = ["EPUB", "PDF", "CBZ"];

/// Typefaces made for long reading, offered in the reader's settings.
const LITERATA: Font = fontsource_font!(LITERATA, weight: [400, 700], style: [Normal, Italic]);
const ATKINSON: Font = fontsource_font!(ATKINSON_HYPERLEGIBLE, weight: [400, 700], style: [Normal, Italic]);
/// Weighted letter bottoms, meant to stop letters flipping for readers with dyslexia.
const OPEN_DYSLEXIC: Font = fontsource_font!(OPENDYSLEXIC, weight: [400, 700], style: [Normal, Italic]);

/// Whether a request path is the reader's, `/books/{id}/read`. The root layout shows
/// the reader alone, without the app's sidebar and header.
pub fn is_reader(path: &str) -> bool {
    matches!(path.split('/').collect::<Vec<_>>().as_slice(), ["", "books", id, "read"] if id.parse::<i64>().is_ok())
}

#[query_params(error = bad_request)]
struct ReadQuery {
    format: Option<String>,
    /// The comic page, from 1.
    n: Option<usize>,
}

/// Reads a book in the browser, full screen.
#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let user = require(cx, Perm::Download).await?.clone();
    let book = library::book(*path_param::<BookId>(cx)?, &hidden_tags(cx).await)?.ok_or_not_found()?;
    let query = query_params::<ReadQuery>(cx)?;
    let format = book
        .formats
        .iter()
        .map(|f| f.format.to_uppercase())
        .filter(|f| READABLE.contains(&f.as_str()))
        .find(|f| query.format.as_ref().is_none_or(|w| w.eq_ignore_ascii_case(f)))
        .ok_or_not_found()?;
    let file = href!(download, BookId(book.id), Format(format.to_lowercase()));
    let here = href!(page, BookId(book.id)).resolve(cx);
    let prefs = Prefs::load(cx, user.id).await?;
    let saved = ReadingPosition::get(cx, user.id, book.id as u64, &format).await?;

    // Comics are read one page image at a time, so no large download is needed. The
    // page shown is the account's place in the comic, so it is saved on every turn.
    let (pages, n) = if format == "CBZ" {
        let (path, _) = library::format_file(book.id, "cbz", &hidden_tags(cx).await)?.ok_or_not_found()?;
        let pages = library::comic_page_count(&path)?;
        let resume = saved.as_ref().and_then(|s| s.position.parse().ok());
        let n = query.n.or(resume).unwrap_or(1).clamp(1, pages.max(1));
        if pages > 0 {
            ReadingPosition::save(cx, user.id, book.id as u64, "CBZ", n.to_string(), (n * 100 / pages) as u64).await?;
        }
        (pages, n)
    } else {
        (0, 1)
    };
    let comic_link = move |n: usize| format!("{here}?format=cbz&n={n}");
    let percent = saved.as_ref().map_or(0, |s| s.percent);
    let epub = format == "EPUB";
    // The browser's own PDF viewer takes no colours or fonts from the page.
    let styled = format != "PDF";
    let authors = book.authors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", ");
    let icon_button = || button_variants(ButtonVariant::Ghost, ButtonSize::Icon);

    Ok(view! {
        topcoat::font::link(font: LITERATA)
        topcoat::font::link(font: ATKINSON)
        topcoat::font::link(font: OPEN_DYSLEXIC)
        <div
            id="reader"
            data-reader-theme=(&prefs.theme)
            data-font=(&prefs.font)
            data-size=(prefs.size)
            data-line-height=(prefs.line_height)
            data-format=(&format)
            data-position-url=(href!(save_position, BookId(book.id)))
            data-prefs-url=(href!(save_prefs, BookId(book.id)))
            class="fixed inset-0 flex flex-col bg-background text-foreground"
        >
            <header class="flex items-center gap-1 border-b border-border px-2 py-1.5 sm:px-3">
                <a href=(href!(book_page, BookId(book.id))) aria-label=(t!(back_to_book)) title=(t!(back_to_book)) class=(icon_button())>
                    icon(data: iconify_icon!("lucide:arrow-left"))
                </a>
                <div class="min-w-0 flex-1 px-1 leading-tight">
                    <h1 class="truncate text-sm font-medium">(&book.title)</h1>
                    <p id="chapter" class="truncate text-xs text-muted-foreground">(&authors)</p>
                </div>
                if epub {
                    <button id="toc-toggle" type="button" aria-expanded="false" aria-controls="toc" aria-label=(t!(reader_contents)) title=(t!(reader_contents)) class=(icon_button())>
                        icon(data: iconify_icon!("lucide:list"))
                    </button>
                }
                if styled {
                    <button id="settings-toggle" type="button" aria-expanded="false" aria-controls="settings" aria-label=(t!(reader_settings)) title=(t!(reader_settings)) class=(icon_button())>
                        icon(data: iconify_icon!("lucide:a-large-small"))
                    </button>
                }
            </header>

            <div class="relative flex min-h-0 flex-1">
                match format.as_str() {
                    "EPUB" => {
                        <button id="prev" type="button" aria-label=(t!(previous_page)) title=(t!(previous_page)) class="reader-turn">
                            icon(data: iconify_icon!("lucide:chevron-left"))
                        </button>
                        <div
                            id="viewer"
                            data-src=(file)
                            data-position=(saved.as_ref().map(|s| s.position.clone()))
                            data-resume-label=(t!(reader_resume))
                            data-resume-dismiss=(t!(reader_resume_dismiss))
                            class="min-w-0 flex-1"
                        ></div>
                        <button id="next" type="button" aria-label=(t!(next_page)) title=(t!(next_page)) class="reader-turn">
                            icon(data: iconify_icon!("lucide:chevron-right"))
                        </button>
                    },
                    "CBZ" => {
                        <a id="prev" href=(comic_link(n.saturating_sub(1).max(1))) aria-label=(t!(previous_page)) title=(t!(previous_page)) class="reader-turn">
                            icon(data: iconify_icon!("lucide:chevron-left"))
                        </a>
                        <div class="flex min-w-0 flex-1 items-center justify-center p-2">
                            if pages == 0 {
                                <p class="text-muted-foreground">(t!(comic_empty))</p>
                            } else {
                                <img
                                    src=(format!("{}?n={}", href!(comic_image, BookId(book.id)).resolve(cx), n))
                                    alt=(t!(page_n, n))
                                    class="max-h-full max-w-full object-contain"
                                >
                            }
                        </div>
                        <a id="next" href=(comic_link((n + 1).min(pages.max(1)))) aria-label=(t!(next_page)) title=(t!(next_page)) class="reader-turn">
                            icon(data: iconify_icon!("lucide:chevron-right"))
                        </a>
                    },
                    _ => <iframe title=(&book.title) src=(file.resolve(cx) + "?inline=1") class="min-w-0 flex-1 bg-card"></iframe>,
                }

                if epub {
                    <nav id="toc" hidden="" aria-label=(t!(reader_contents)) class="reader-panel left-2 sm:left-3">
                        <h2 class="px-2 pb-2 text-sm font-medium">(t!(reader_contents))</h2>
                        <ol id="toc-list" class="grid gap-0.5 text-sm"></ol>
                    </nav>
                }

                if styled {
                    <form id="settings" hidden="" aria-label=(t!(reader_settings)) class="reader-panel right-2 grid gap-5 sm:right-3">
                        <fieldset>
                            <legend class="mb-2 text-sm font-medium">(t!(reader_theme))</legend>
                            <div class="grid grid-cols-4 gap-2">
                                for theme in THEMES {
                                    <label class="reader-swatch" data-reader-theme=(theme)>
                                        <input type="radio" name="theme" value=(theme) checked=((prefs.theme == theme).then_some("")) class="sr-only">
                                        <span aria-hidden="true" class="text-base">"Aa"</span>
                                        <span class="text-xs">(theme_label(theme))</span>
                                    </label>
                                }
                            </div>
                        </fieldset>
                        if epub {
                            <label class="grid gap-2 text-sm font-medium">
                                (t!(reader_font))
                                <select name="font" class="h-9 rounded-lg border border-border bg-card px-2 text-sm font-normal">
                                    for font in FONTS {
                                        <option value=(font) selected=((prefs.font == font).then_some(""))>(font_label(font))</option>
                                    }
                                </select>
                            </label>
                            <label class="grid gap-2 text-sm font-medium">
                                <span class="flex justify-between">(t!(reader_size)) <output id="size-value" class="font-normal text-muted-foreground tabular-nums">(prefs.size) "%"</output></span>
                                <input type="range" name="size" min="70" max="200" step="10" value=(prefs.size) class="accent-primary">
                            </label>
                            <label class="grid gap-2 text-sm font-medium">
                                <span class="flex justify-between">(t!(reader_line_height)) <output id="line-height-value" class="font-normal text-muted-foreground tabular-nums">(format!("{:.1}", prefs.line_height as f64 / 100.0))</output></span>
                                <input type="range" name="line_height" min="120" max="220" step="10" value=(prefs.line_height) class="accent-primary">
                            </label>
                        }
                        <p class="text-xs text-muted-foreground">(if epub { t!(reader_keys) } else { t!(reader_keys_comic) })</p>
                    </form>
                }
            </div>

            <footer class="flex items-center gap-3 border-t border-border px-3 py-2 text-xs text-muted-foreground sm:px-4">
                match format.as_str() {
                    "EPUB" => {
                        // Filled by `reader.js` once the book is measured.
                        <span
                            id="page"
                            data-template=(t!(reader_page_of, n = "{n}", total = "{total}"))
                            class="whitespace-nowrap tabular-nums"
                        ></span>
                        <input
                            id="progress"
                            type="range"
                            min="0"
                            max="100"
                            step="1"
                            value=(percent)
                            disabled=""
                            aria-label=(t!(reader_progress))
                            class="min-w-0 flex-1 accent-primary"
                        >
                        <span id="percent" class="w-10 text-right tabular-nums">(percent) "%"</span>
                    },
                    "CBZ" => <span class="mx-auto tabular-nums">(t!(page_n, n)) " / " (pages)</span>,
                    _ => <span class="mx-auto">(t!(reader_pdf_note))</span>,
                }
            </footer>
        </div>
        if epub {
            <script src=(asset!("https://cdn.jsdelivr.net/npm/jszip@3.10.1/dist/jszip.min.js"))></script>
            <script src=(asset!("https://cdn.jsdelivr.net/npm/epubjs@0.3.93/dist/epub.min.js"))></script>
        }
        <script src=(asset!("./reader.js"))></script>
    })
}

fn theme_label(theme: &str) -> topcoat::l10n::Translation<'static> {
    match theme {
        "sepia" => t!(theme_sepia),
        "white" => t!(theme_white),
        "dark" => t!(theme_dark),
        _ => t!(theme_cream),
    }
}

fn font_label(font: &str) -> topcoat::l10n::Translation<'static> {
    match font {
        "publisher" => t!(font_publisher),
        "serif" => t!(font_serif),
        "sans" => t!(font_sans),
        "atkinson" => t!(font_atkinson),
        "dyslexic" => t!(font_dyslexic),
        _ => t!(font_literata),
    }
}

#[derive(Deserialize)]
struct PositionBody {
    format: String,
    position: String,
    /// Unknown until the reader has measured the book; the stored one is kept then.
    percent: Option<u64>,
}

/// Saves the reader's place in the book on the account.
#[route(PUT "./position")]
pub async fn save_position(cx: &Cx, Json(body): Json<PositionBody>) -> Result<StatusCode> {
    let user = require(cx, Perm::Download).await?.clone();
    let book = library::book(*path_param::<BookId>(cx)?, &hidden_tags(cx).await)?.ok_or_not_found()?;
    let format = body.format.to_uppercase();
    if !book.formats.iter().any(|f| f.format.eq_ignore_ascii_case(&format)) || body.position.len() > 2048 {
        return Err(bad_request("unknown format, or position too long").into());
    }
    let percent = match body.percent {
        Some(percent) => percent,
        None => ReadingPosition::get(cx, user.id, book.id as u64, &format).await?.map_or(0, |s| s.percent),
    };
    ReadingPosition::save(cx, user.id, book.id as u64, &format, body.position, percent).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct PrefsBody {
    theme: String,
    font: String,
    size: u64,
    line_height: u64,
}

/// Saves how the user likes the reader's text shown, for every book.
#[route(PUT "./prefs")]
pub async fn save_prefs(cx: &Cx, Json(body): Json<PrefsBody>) -> Result<StatusCode> {
    let user = require(cx, Perm::Download).await?.clone();
    let PrefsBody { theme, font, size, line_height } = body;
    Prefs { theme, font, size, line_height }.save(cx, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// One page image of the book's CBZ comic, `?n=` from 1.
#[route(GET "./page")]
pub async fn comic_image(cx: &Cx) -> Result<([(header::HeaderName, &'static str); 2], Vec<u8>)> {
    require(cx, Perm::Download).await?;
    let n = query_params::<ReadQuery>(cx)?.n.unwrap_or(1).max(1);
    let (path, _) = library::format_file(*path_param::<BookId>(cx)?, "cbz", &hidden_tags(cx).await)?.ok_or_not_found()?;
    let (bytes, kind) = library::comic_page(&path, n - 1)?.ok_or_not_found()?;
    Ok(([(header::CONTENT_TYPE, kind), (header::CACHE_CONTROL, "private, max-age=3600")], bytes))
}

#[cfg(test)]
mod tests {
    #[test]
    fn recognises_reader_paths() {
        assert!(super::is_reader("/books/12/read"));
        assert!(!super::is_reader("/books/12/read/page"));
        assert!(!super::is_reader("/books/12"));
        assert!(!super::is_reader("/books/x/read"));
    }
}
