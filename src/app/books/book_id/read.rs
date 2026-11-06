use topcoat::{
    Result,
    asset::asset,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    router::{error::RouterErrorExt, header, href, page, path_param, query_params, route},
    view::{View, view},
};

use super::{BookId, download::format::{Format, download}, page as book_page};
use crate::{
    app::{hidden_tags, require},
    components::button::{ButtonSize, ButtonVariant, button_variants},
    features::{auth::Perm, library},
    t,
};

/// Formats the browser can show: EPUB through epub.js, PDF natively, CBZ page by page.
pub const READABLE: [&str; 3] = ["EPUB", "PDF", "CBZ"];

#[query_params(error = bad_request)]
struct ReadQuery {
    format: Option<String>,
    /// The comic page, from 1.
    n: Option<usize>,
}

/// Reads a book in the browser.
#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    require(cx, Perm::Download).await?;
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

    // Comics are read one page image at a time, so no large download is needed.
    let (pages, n) = if format == "CBZ" {
        let (path, _) = library::format_file(book.id, "cbz", &hidden_tags(cx).await)?.ok_or_not_found()?;
        let pages = library::comic_page_count(&path)?;
        (pages, query.n.unwrap_or(1).clamp(1, pages.max(1)))
    } else {
        (0, 1)
    };
    let comic_link = move |n: usize| format!("{here}?format=cbz&n={n}");

    Ok(view! {
        <div class="mb-4 flex items-center justify-between gap-4">
            <a href=(href!(book_page, BookId(book.id))) class="flex min-w-0 items-center gap-2 text-sm font-medium hover:underline">
                icon(data: iconify_icon!("lucide:arrow-left"))
                <span class="truncate">(&book.title)</span>
            </a>
            match format.as_str() {
                "EPUB" => <div class="flex gap-2">
                    <button id="prev" type="button" aria-label=(t!(previous_page)) class=(button_variants(ButtonVariant::Outline, ButtonSize::Icon))>
                        icon(data: iconify_icon!("lucide:chevron-left"))
                    </button>
                    <button id="next" type="button" aria-label=(t!(next_page)) class=(button_variants(ButtonVariant::Outline, ButtonSize::Icon))>
                        icon(data: iconify_icon!("lucide:chevron-right"))
                    </button>
                </div>,
                "CBZ" => <div class="flex items-center gap-2">
                    <a href=(comic_link(n.saturating_sub(1).max(1))) aria-label=(t!(previous_page)) class=(button_variants(ButtonVariant::Outline, ButtonSize::Icon))>
                        icon(data: iconify_icon!("lucide:chevron-left"))
                    </a>
                    <span class="text-sm text-muted-foreground tabular-nums">(n) " / " (pages)</span>
                    <a href=(comic_link((n + 1).min(pages.max(1)))) aria-label=(t!(next_page)) class=(button_variants(ButtonVariant::Outline, ButtonSize::Icon))>
                        icon(data: iconify_icon!("lucide:chevron-right"))
                    </a>
                </div>,
                _ => "",
            }
        </div>
        match format.as_str() {
            "EPUB" => {
                <div id="viewer" data-src=(file) class="h-[calc(100vh-11rem)] overflow-hidden rounded-xl border border-border bg-card shadow-sm"></div>
                <script src=(asset!("https://cdn.jsdelivr.net/npm/jszip@3.10.1/dist/jszip.min.js"))></script>
                <script src=(asset!("https://cdn.jsdelivr.net/npm/epubjs@0.3.93/dist/epub.min.js"))></script>
                <script src=(asset!("./reader.js"))></script>
            },
            "CBZ" => {
                if pages == 0 {
                    <p class="text-muted-foreground">(t!(comic_empty))</p>
                } else {
                    // Clicking the page turns it, like a comic app.
                    <a href=(comic_link((n + 1).min(pages))) class="block">
                        <img
                            src=(format!("{}?n={}", href!(comic_image, BookId(book.id)).resolve(cx), n))
                            alt=(t!(page_n, n))
                            class="mx-auto max-h-[calc(100vh-11rem)] rounded-lg shadow-sm"
                        >
                    </a>
                }
            },
            _ => <iframe
                title=(&book.title)
                src=(file.resolve(cx) + "?inline=1")
                class="h-[calc(100vh-11rem)] w-full rounded-xl border border-border bg-card shadow-sm"
            ></iframe>,
        }
    })
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
