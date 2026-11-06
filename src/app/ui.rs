//! Book list pieces shared by the library pages.

use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    l10n::Translation,
    router::{href, query_params},
    view::{Child, Length, View, attributes, class, component, view},
};

use super::books::book_id::{BookId, cover::cover as cover_route, page as book_page};
use crate::{
    components::button::{ButtonSize, ButtonVariant, button},
    features::library::{self, BookSummary, Filter, PAGE_SIZE, SORTS},
    t,
};

/// The query string of a book list: the page, the sort order, and a search.
#[query_params(error = bad_request)]
pub struct ListQuery {
    pub page: Option<i64>,
    pub sort: Option<String>,
    pub q: Option<String>,
}

/// The outcome of a form, carried to the page it redirects to.
#[query_params(error = bad_request)]
pub struct NoticeQuery {
    pub msg: Option<String>,
    pub error: Option<String>,
}

/// `url` with a success message for [`notice`] to show.
pub fn with_msg(url: String, text: &str) -> String {
    format!("{url}?msg={}", encode(text))
}

/// `url` with an error message for [`notice`] to show.
pub fn with_error(url: String, text: &str) -> String {
    format!("{url}?error={}", encode(text))
}

/// A banner for the `?msg=` or `?error=` of the current page.
#[component]
pub async fn notice(cx: &Cx) -> Result<impl View> {
    let query = query_params::<NoticeQuery>(cx).ok();
    let msg = query.and_then(|q| q.msg.clone());
    let error = query.and_then(|q| q.error.clone());
    Ok(view! {
        match error {
            Some(error) => <p role="alert" class="mb-6 flex items-start gap-2 rounded-lg border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm">
                icon(data: iconify_icon!("lucide:circle-alert"), attrs: attributes! { class="mt-0.5 shrink-0 text-destructive" })
                (error)
            </p>,
            None => "",
        }
        match msg {
            Some(msg) => <p role="status" class="mb-6 flex items-start gap-2 rounded-lg border border-emerald-500/40 bg-emerald-500/10 px-4 py-3 text-sm">
                icon(data: iconify_icon!("lucide:circle-check"), attrs: attributes! { class="mt-0.5 shrink-0 text-emerald-600" })
                (msg)
            </p>,
            None => "",
        }
    })
}

/// A labelled form control. Pass the control as the child.
#[component]
pub async fn field(
    #[into] label: String,
    #[default]
    #[into]
    hint: String,
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <label class="grid gap-1.5 text-sm">
            <span class="font-medium">(label)</span>
            (child)
            if !hint.is_empty() {
                <span class="text-xs text-muted-foreground">(hint)</span>
            }
        </label>
    })
}

/// A checkbox with its label.
#[component]
pub async fn checkbox(name: &str, label: Translation<'static>, checked: bool) -> Result<impl View> {
    Ok(view! {
        <label class="flex items-center gap-2 text-sm">
            <input type="checkbox" name=(name) checked=(checked) class="size-4 accent-primary">
            (label)
        </label>
    })
}

/// A page heading with an optional line under it.
#[component]
pub async fn heading(
    #[into] title: String,
    #[default]
    #[into]
    subtitle: String,
) -> Result<impl View> {
    Ok(view! {
        <div>
            <h1 class="text-2xl font-semibold tracking-tight md:text-3xl">(title)</h1>
            if !subtitle.is_empty() {
                <p class="mt-1 text-sm text-muted-foreground">(subtitle)</p>
            }
        </div>
    })
}

/// The label of a sort order from [`SORTS`].
fn sort_label(key: &str) -> Translation<'static> {
    match key {
        "old" => t!(sort_old),
        "title" => t!(sort_title),
        "author" => t!(sort_author),
        "pubdate" => t!(sort_pubdate),
        _ => t!(sort_new),
    }
}

/// A titled, sortable, paginated grid of the books matching `filter`.
#[component]
pub async fn book_list(
    cx: &Cx,
    title: String,
    filter: Filter<'_>,
    default_sort: &'static str,
) -> Result<impl View> {
    let query = query_params::<ListQuery>(cx)?;
    let sort = query.sort.as_deref().unwrap_or(default_sort);
    let page = query.page.unwrap_or(1).max(1);
    let (books, total) = library::books(&filter, sort, page, &super::hidden_tags(cx).await)?;

    Ok(view! {
        <div class="mb-8 flex flex-wrap items-end justify-between gap-4">
            <div>
                <h1 class="text-2xl font-semibold tracking-tight md:text-3xl">(title)</h1>
                <p class="mt-1 text-sm text-muted-foreground">(t!(book_count, count = total))</p>
            </div>
            <nav aria-label=(t!(sort_label)) class="flex flex-wrap gap-1 rounded-lg border border-border bg-card p-1">
                for key in SORTS {
                    <a
                        href=(sort_href(query.q.as_deref(), key))
                        aria-current=((sort == key).then_some("true"))
                        class=(class!(
                            "rounded-md px-3 py-1 text-xs font-medium transition-colors",
                            "bg-primary text-primary-foreground" if sort == key,
                            "text-muted-foreground hover:bg-foreground/5 hover:text-foreground" if sort != key,
                        ))
                    >
                        (sort_label(key))
                    </a>
                }
            </nav>
        </div>
        book_grid(books: books)
        pager(page: page, total: total, sort: query.sort.clone(), q: query.q.clone())
    })
}

/// A relative link to the current list in another sort order, keeping the search.
fn sort_href(q: Option<&str>, sort: &str) -> String {
    match q {
        Some(q) => format!("?sort={sort}&q={}", encode(q)),
        None => format!("?sort={sort}"),
    }
}

/// Percent-encodes a query value.
pub fn encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// A responsive grid of book covers with their titles and authors.
#[component]
pub async fn book_grid(books: Vec<BookSummary>) -> Result<impl View> {
    Ok(view! {
        if books.is_empty() {
            <div class="flex flex-col items-center gap-3 rounded-xl border border-dashed border-border py-20 text-center text-muted-foreground">
                icon(data: iconify_icon!("lucide:book-dashed"), size: Length::px(32.0))
                <p>(t!(no_books))</p>
            </div>
        } else {
            <ul class="grid grid-cols-2 gap-x-5 gap-y-8 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6">
                #[key(book.id)]
                for book in books {
                    <li>
                        <a href=(href!(book_page, BookId(book.id))) class="group block outline-none">
                            cover(id: book.id, title: book.title.clone(), has_cover: book.has_cover)
                            <p class="mt-3 line-clamp-2 text-sm leading-snug font-medium group-hover:underline group-focus-visible:underline">
                                (&book.title)
                            </p>
                            <p class="mt-0.5 line-clamp-1 text-xs text-muted-foreground">(&book.authors)</p>
                            match &book.series {
                                Some(series) => <p class="mt-0.5 line-clamp-1 text-xs text-muted-foreground/80">
                                    (series) " · #" (series_number(book.series_index))
                                </p>,
                                None => "",
                            }
                        </a>
                    </li>
                }
            </ul>
        }
    })
}

/// A series index without a trailing `.0`.
pub fn series_number(index: f64) -> String {
    if index.fract() == 0.0 { format!("{index:.0}") } else { index.to_string() }
}

/// A book cover at a 2:3 ratio. Books without a cover get a generated one.
#[component]
pub async fn cover(id: i64, title: String, has_cover: bool) -> Result<impl View> {
    // Spread the placeholder hues so neighbouring books differ.
    let hue = (id * 47) % 360;
    Ok(view! {
        <div class="aspect-[2/3] overflow-hidden rounded-lg shadow-sm ring-1 ring-border transition duration-200 group-hover:-translate-y-1 group-hover:shadow-lg">
            if has_cover {
                <img src=(href!(cover_route, BookId(id))) alt="" loading="lazy" class="size-full bg-foreground/5 object-cover">
            } else {
                <div
                    class="flex size-full flex-col justify-between p-4 text-white"
                    style=(format!(
                        "background: linear-gradient(155deg, oklch(0.6 0.12 {hue}), oklch(0.36 0.09 {}))",
                        hue + 40
                    ))
                >
                    <span class="line-clamp-6 text-base leading-tight font-semibold [text-shadow:0_1px_2px_rgb(0_0_0/30%)]">(title)</span>
                    <span class="opacity-70">icon(data: iconify_icon!("lucide:book"), size: Length::px(20.0))</span>
                </div>
            }
        </div>
    })
}

/// Previous and next buttons for a list of `total` books, keeping its query.
#[component]
async fn pager(page: i64, total: i64, sort: Option<String>, q: Option<String>) -> Result<impl View> {
    let pages = (total + PAGE_SIZE - 1) / PAGE_SIZE;
    Ok(view! {
        if pages > 1 {
            // A GET form lets the browser encode the kept query values.
            <form class="mt-12 flex items-center justify-center gap-3">
                match sort { Some(sort) => <input type="hidden" name="sort" value=(sort)>, None => "" }
                match q { Some(q) => <input type="hidden" name="q" value=(q)>, None => "" }
                button(
                    variant: ButtonVariant::Outline,
                    size: ButtonSize::Sm,
                    attrs: attributes! { name="page" value=(page - 1) disabled=(page <= 1) },
                    icon(data: iconify_icon!("lucide:chevron-left"))
                    (t!(previous))
                )
                <span class="text-sm text-muted-foreground tabular-nums">(t!(page_of, page, pages))</span>
                button(
                    variant: ButtonVariant::Outline,
                    size: ButtonSize::Sm,
                    attrs: attributes! { name="page" value=(page + 1) disabled=(page >= pages) },
                    (t!(next))
                    icon(data: iconify_icon!("lucide:chevron-right"))
                )
            </form>
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_query_values() {
        assert_eq!(encode("città & co"), "citt%C3%A0%20%26%20co");
        assert_eq!(sort_href(None, "title"), "?sort=title");
    }

    #[test]
    fn formats_series_numbers() {
        assert_eq!(series_number(2.0), "2");
        assert_eq!(series_number(2.5), "2.5");
    }
}
