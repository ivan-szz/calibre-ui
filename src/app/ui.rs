//! Book list pieces shared by the library pages.

use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    l10n::Translation,
    router::{href, query_params},
    view::{Child, Length, View, attributes, class, component, view},
};

use super::books::book_id::{BookId, cover::cover as cover_route, page as book_page, read};
use crate::{
    components::button::{ButtonSize, ButtonVariant, button},
    features::{
        library::{self, BookSummary, Filter, PAGE_SIZE, SORTS},
        reader::ReadingPosition,
        shelf::ReadMark,
    },
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
            <h1 class="font-serif text-3xl font-medium md:text-4xl">(title)</h1>
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
                <h1 class="font-serif text-3xl font-medium md:text-4xl">(title)</h1>
                <p class="mt-1 text-sm text-muted-foreground">(t!(book_count, count = total))</p>
            </div>
            // One row that scrolls sideways when it does not fit, like the top navigation.
            <nav aria-label=(t!(sort_label)) class="flex max-w-full gap-1 overflow-x-auto rounded-lg border border-border bg-card p-1 [scrollbar-width:none]">
                for key in SORTS {
                    <a
                        href=(sort_href(query.q.as_deref(), key))
                        aria-current=((sort == key).then_some("true"))
                        class=(class!(
                            // The bar's 8px radius minus its 4px padding.
                            "shrink-0 rounded px-3 py-1 text-xs font-medium whitespace-nowrap transition-colors",
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
pub async fn book_grid(cx: &Cx, books: Vec<BookSummary>) -> Result<impl View> {
    // The signed-in user's own marks: a gilt ribbon on books they have read, a gilt
    // progress line on books they are partway through. Unread books carry nothing.
    let user_id = super::signed_in(cx).await?.id;
    let read: std::collections::HashSet<u64> = ReadMark::book_ids(cx, user_id).await?.into_iter().collect();
    let progress: std::collections::HashMap<u64, u64> =
        ReadingPosition::in_progress(cx, user_id).await?.into_iter().map(|p| (p.book_id, p.percent)).collect();
    let books: Vec<(BookSummary, bool, Option<u64>)> = books
        .into_iter()
        .map(|b| {
            let id = b.id as u64;
            let done = read.contains(&id);
            let percent = if done { None } else { progress.get(&id).copied() };
            (b, done, percent)
        })
        .collect();
    Ok(view! {
        if books.is_empty() {
            <div class="flex flex-col items-center gap-3 rounded-xl border border-dashed border-border py-20 text-center text-muted-foreground">
                icon(data: iconify_icon!("lucide:book-dashed"), size: Length::px(32.0))
                <p>(t!(no_books))</p>
            </div>
        } else {
            <ul class="grid grid-cols-2 gap-x-5 gap-y-8 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6">
                #[key(book.id)]
                for (book, done, percent) in books {
                    <li>
                        <a href=(href!(book_page, BookId(book.id))) class="group block outline-none">
                            <div class="relative">
                                cover(id: book.id, title: book.title.clone(), has_cover: book.has_cover)
                                // On hover or keyboard focus, the cover dims under where the
                                // reader is. The ribbon and bar stay the signs for touch
                                // screens, and come after it so the dimming leaves them bright.
                                if done || percent.is_some() {
                                    <div
                                        aria-hidden="true"
                                        class="pointer-events-none absolute inset-0 flex flex-col items-center justify-center gap-1 rounded-[3px] bg-black/60 text-white opacity-0 transition-opacity group-hover:opacity-100 group-focus-visible:opacity-100"
                                    >
                                        match percent {
                                            Some(percent) => {
                                                <span class="font-serif text-4xl font-medium tabular-nums">(percent) "%"</span>
                                                <span class="text-xs text-white/75">(t!(read_so_far))</span>
                                            },
                                            None => {
                                                icon(data: iconify_icon!("lucide:check"), size: Length::px(28.0))
                                                <span class="font-serif text-lg font-medium">(t!(read_badge))</span>
                                            },
                                        }
                                    </div>
                                }
                                if done {
                                    // A ribbon bookmark hanging from the top edge, notched at its tail.
                                    <span
                                        role="img"
                                        aria-label=(t!(read_badge))
                                        title=(t!(read_badge))
                                        class="absolute top-0 right-3 h-11 w-6 bg-gilt drop-shadow-[0_1px_1px_rgb(0_0_0/35%)] [clip-path:polygon(0_0,100%_0,100%_100%,50%_78%,0_100%)]"
                                    ></span>
                                }
                                match percent {
                                    Some(percent) => <div
                                        role="progressbar"
                                        aria-valuemin="0"
                                        aria-valuemax="100"
                                        aria-valuenow=(percent)
                                        aria-label=(t!(percent_read, percent))
                                        title=(t!(percent_read, percent))
                                        class="absolute inset-x-0 bottom-0 h-1.5 overflow-hidden rounded-b-[3px] bg-black/45"
                                    >
                                        <div class="h-full bg-gilt" style=(format!("width: {percent}%"))></div>
                                    </div>,
                                    None => "",
                                }
                            </div>
                            <p class="mt-3 line-clamp-2 text-sm leading-snug font-medium group-hover:underline group-focus-visible:underline">
                                (&book.title)
                            </p>
                            <p class="mt-0.5 line-clamp-1 text-xs text-muted-foreground">(&book.authors)</p>
                            match &book.series {
                                Some(series) => <p class="mt-0.5 line-clamp-1 text-xs text-muted-foreground/80">
                                    (series) " #" (series_number(book.series_index))
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

/// Bookcloth colours for generated covers: green, oxblood, navy, ochre, slate, plum.
const CLOTH: [&str; 6] = ["#2f5446", "#6b2c2c", "#273a5c", "#7d5f26", "#46525c", "#553653"];

/// A book cover at a 2:3 ratio. Books without a cover get a clothbound one, with the
/// title in gilt inside a blind-stamped frame.
#[component]
pub async fn cover(id: i64, title: String, has_cover: bool) -> Result<impl View> {
    // Consecutive ids get different cloths, so neighbouring books differ.
    let cloth = CLOTH[id.rem_euclid(CLOTH.len() as i64) as usize];
    Ok(view! {
        <div class="@container aspect-[2/3] overflow-hidden rounded-[3px] shadow-sm ring-1 ring-foreground/10 transition-shadow group-hover:ring-2 group-hover:ring-primary group-focus-visible:ring-2 group-focus-visible:ring-ring">
            if has_cover {
                <img src=(href!(cover_route, BookId(id))) alt="" loading="lazy" class="size-full bg-foreground/5 object-cover">
            } else {
                <div
                    class="relative flex size-full items-center justify-center px-5 text-center"
                    style=(format!("background-color: {cloth}; box-shadow: inset 0 0 40px rgb(0 0 0 / 25%)"))
                >
                    <span aria-hidden="true" class="absolute inset-y-0 left-2 w-px bg-black/30"></span>
                    // Thumbnails keep only the cloth and spine; the frame and title need room.
                    <span aria-hidden="true" class="absolute inset-y-3 right-3 left-5 hidden rounded-[2px] border border-[#e8c983]/35 @[6rem]:block"></span>
                    <span class="relative line-clamp-5 hidden font-serif text-[15px] leading-snug font-medium text-balance text-[#ecd193] @[6rem]:block">(title)</span>
                </div>
            }
        </div>
    })
}

/// The books the user is partway through, most recently read first, each opening
/// straight into the reader where they left off. Only on the first page of a list.
#[component]
pub async fn continue_reading(cx: &Cx) -> Result<impl View> {
    let user_id = super::signed_in(cx).await?.id;
    let first_page = query_params::<ListQuery>(cx)?.page.unwrap_or(1) <= 1;
    let read = ReadMark::book_ids(cx, user_id).await?;
    let mut positions = ReadingPosition::in_progress(cx, user_id).await?;
    positions.retain(|p| !read.contains(&p.book_id));
    positions.truncate(6);
    let ids: Vec<u64> = positions.iter().map(|p| p.book_id).collect();
    let books = if first_page && !ids.is_empty() {
        library::books(&Filter::Ids(&ids), "new", 1, &super::hidden_tags(cx).await)?.0
    } else {
        Vec::new()
    };
    // Back in reading order; books hidden from the user drop out here.
    let items: Vec<(BookSummary, String, u64)> = positions
        .iter()
        .filter_map(|p| {
            let book = books.iter().find(|b| b.id as u64 == p.book_id)?.clone();
            Some((book, p.format.to_lowercase(), p.percent))
        })
        .collect();

    Ok(view! {
        if !items.is_empty() {
            <section aria-labelledby="reading-now" class="mb-12">
                <h2 id="reading-now" class="mb-4 font-serif text-xl font-medium">(t!(reading_now))</h2>
                // Phones scroll the row sideways, so the library stays near the top.
                <ul class="-mx-4 flex snap-x snap-mandatory scroll-px-4 gap-3 overflow-x-auto px-4 pb-2 sm:mx-0 sm:grid sm:grid-cols-2 sm:overflow-visible sm:px-0 sm:pb-0 xl:grid-cols-3">
                    #[key(book.id)]
                    for (book, format, percent) in items {
                        <li class="w-[85%] shrink-0 snap-start sm:w-auto">
                            <a
                                href=(href!(read::page, BookId(book.id)).resolve(cx) + "?format=" + &format)
                                class="group flex items-center gap-4 rounded-xl border border-border bg-card p-3 outline-none transition-colors hover:border-primary/60 focus-visible:ring-2 focus-visible:ring-ring"
                            >
                                <div class="w-14 shrink-0">
                                    cover(id: book.id, title: book.title.clone(), has_cover: book.has_cover)
                                </div>
                                <div class="min-w-0 flex-1">
                                    <p class="line-clamp-2 font-serif leading-snug font-medium">(&book.title)</p>
                                    <p class="mt-0.5 line-clamp-1 text-xs text-muted-foreground">(&book.authors)</p>
                                    <div class="mt-3 flex items-center gap-2 text-xs text-muted-foreground tabular-nums">
                                        <div
                                            role="progressbar"
                                            aria-valuemin="0"
                                            aria-valuemax="100"
                                            aria-valuenow=(percent)
                                            aria-label=(t!(percent_read, percent))
                                            class="h-1 flex-1 overflow-hidden rounded-full bg-foreground/10"
                                        >
                                            <div class="h-full rounded-full bg-gilt" style=(format!("width: {percent}%"))></div>
                                        </div>
                                        (percent) "%"
                                    </div>
                                </div>
                            </a>
                        </li>
                    }
                </ul>
            </section>
        }
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
