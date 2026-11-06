pub mod cover;
pub mod download;
pub mod edit;
pub mod read;

use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    router::{
        content::Form,
        error::{RouterErrorExt, SeeOther, see_other},
        href, module_param, page, path_param, route,
    },
    runtime::{Event, Signal, procedure, shard, signal},
    view::{View, attributes, component, view},
};

use crate::{
    app::{
        browse::category::{Category, item_id::{ItemId, page as item_page}},
        hidden_tags, require, shelves, signed_in,
        tasks,
        ui::{cover as book_cover, series_number, with_error, with_msg},
    },
    components::button::{ButtonSize, ButtonVariant, button, button_variants},
    features::{
        auth::Perm,
        calibre::{self, OUTPUT_FORMATS},
        library::{self, Item, Kind, content_type, human_size},
        mailer,
        shelf::{ReadMark, Shelf},
    },
    t,
};
use download::format::{Format, download as download_file};

module_param!(pub book_id: i64, error = not_found);

/// Formats worth emailing to an e-reader, best first.
const SENDABLE: [&str; 5] = ["EPUB", "AZW3", "MOBI", "PDF", "TXT"];

/// The details of one book, its downloads, and its shelves.
#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let id = *path_param::<BookId>(cx)?;
    let book = library::book(id, &hidden_tags(cx).await)?.ok_or_not_found()?;
    let user = signed_in(cx).await?.clone();
    let revision = signal(cx, || 0u32);
    let read_revision = signal(cx, || 0u32);
    let rating = book.ratings.first().map(|r| Kind::Ratings.display(&r.name));
    let has = |f: &str| book.formats.iter().any(|b| b.format.eq_ignore_ascii_case(f));
    let readable: Vec<&str> = read::READABLE.into_iter().filter(|f| has(f)).collect();
    let sendable: Vec<&str> = SENDABLE.into_iter().filter(|f| has(f)).collect();
    let targets: Vec<&str> = OUTPUT_FORMATS.into_iter().filter(|f| !has(f)).collect();
    let can_download = user.can(Perm::Download);
    let can_edit = user.can(Perm::Edit);

    Ok(view! {
        <article class="grid gap-8 md:grid-cols-[minmax(0,240px)_1fr] lg:gap-12">
            <div class="mx-auto w-48 md:mx-0 md:w-full">
                <div class="group">
                    book_cover(id: book.id, title: book.title.clone(), has_cover: book.has_cover)
                </div>
                if can_download && !readable.is_empty() {
                    <div class="mt-6 grid gap-2">
                        for format in &readable {
                            <a
                                href=(href!(read::page, BookId(id)).resolve(cx) + "?format=" + &format.to_lowercase())
                                class=(button_variants(ButtonVariant::Primary, ButtonSize::Md))
                            >
                                icon(data: iconify_icon!("lucide:book-open-text"))
                                (t!(read_book)) " " (if readable.len() > 1 { *format } else { "" })
                            </a>
                        }
                    </div>
                }
                <div class="mt-2">read_button(book_id: id as u64, revision: read_revision)</div>
                if can_edit {
                    <a href=(href!(edit::page, BookId(id))) class=(button_variants(ButtonVariant::Outline, ButtonSize::Md)) style="margin-top: 0.5rem; width: 100%">
                        icon(data: iconify_icon!("lucide:pencil"))
                        (t!(edit))
                    </a>
                }
                if can_download && !book.formats.is_empty() {
                    <h2 class="mt-6 mb-2 text-xs font-medium tracking-wide text-muted-foreground uppercase">(t!(download))</h2>
                    <ul class="space-y-2">
                        for format in &book.formats {
                            <li>
                                <a
                                    href=(href!(download_file, BookId(id), Format(format.format.to_lowercase())))
                                    class=(button_variants(ButtonVariant::Outline, ButtonSize::Md))
                                    style="width: 100%; justify-content: space-between"
                                >
                                    <span class="flex items-center gap-2">
                                        icon(data: iconify_icon!("lucide:download"))
                                        (&format.format)
                                    </span>
                                    <span class="text-xs text-muted-foreground">(human_size(format.size))</span>
                                </a>
                            </li>
                        }
                    </ul>
                }
            </div>

            <div class="min-w-0">
                for series in &book.series {
                    <a
                        href=(href!(item_page, Category(Kind::Series), ItemId(series.id.clone())))
                        class="mb-2 inline-flex items-center gap-1.5 text-sm font-medium text-muted-foreground hover:text-foreground"
                    >
                        icon(data: iconify_icon!("lucide:layers"))
                        (t!(book_of_series, number = series_number(book.series_index), series = series.name.as_str()))
                    </a>
                }
                <h1 class="text-3xl font-semibold tracking-tight text-balance md:text-4xl">(&book.title)</h1>
                <p class="mt-3 text-lg text-muted-foreground">
                    (t!(by)) " "
                    item_links(kind: Kind::Authors, items: book.authors.clone())
                </p>
                match rating {
                    Some(stars) => <p class="mt-3 text-lg tracking-widest text-amber-500" aria-label=(t!(rating))>(stars)</p>,
                    None => "",
                }

                if !book.tags.is_empty() {
                    <ul class="mt-5 flex flex-wrap gap-2">
                        for tag in &book.tags {
                            <li>
                                <a
                                    href=(href!(item_page, Category(Kind::Tags), ItemId(tag.id.clone())))
                                    class="inline-flex rounded-full bg-foreground/5 px-3 py-1 text-xs font-medium transition-colors hover:bg-foreground/10"
                                >
                                    (&tag.name)
                                </a>
                            </li>
                        }
                    </ul>
                }

                <dl class="mt-8 grid grid-cols-[auto_1fr] gap-x-8 gap-y-2 text-sm">
                    if !book.publishers.is_empty() {
                        <dt class="text-muted-foreground">(t!(publisher))</dt>
                        <dd>item_links(kind: Kind::Publishers, items: book.publishers.clone())</dd>
                    }
                    match &book.published {
                        Some(date) => {
                            <dt class="text-muted-foreground">(t!(published))</dt>
                            <dd>(date)</dd>
                        },
                        None => "",
                    }
                    if !book.languages.is_empty() {
                        <dt class="text-muted-foreground">(t!(language))</dt>
                        <dd>item_links(kind: Kind::Languages, items: book.languages.clone())</dd>
                    }
                    for (kind, value) in &book.identifiers {
                        <dt class="text-muted-foreground">(kind.to_uppercase())</dt>
                        <dd class="font-mono text-xs leading-5">(value)</dd>
                    }
                    for field in book.custom.iter().filter(|f| !f.value.is_empty()) {
                        <dt class="text-muted-foreground">(&field.name)</dt>
                        <dd class="whitespace-pre-line">(field.display())</dd>
                    }
                </dl>

                if !book.description.is_empty() {
                    <section class="mt-8 max-w-prose space-y-3 leading-relaxed">
                        for paragraph in &book.description {
                            <p>(paragraph)</p>
                        }
                    </section>
                }

                <section class="mt-10 border-t border-border pt-6">
                    <h2 class="mb-3 flex items-center gap-2 text-sm font-semibold">
                        icon(data: iconify_icon!("lucide:bookmark"))
                        (t!(nav_shelves))
                    </h2>
                    shelf_buttons(book_id: id as u64, revision: revision)
                </section>

                if can_download && !sendable.is_empty() {
                    <section class="mt-8 border-t border-border pt-6">
                        <h2 class="mb-3 flex items-center gap-2 text-sm font-semibold">
                            icon(data: iconify_icon!("lucide:send"))
                            (t!(send_to_ereader))
                        </h2>
                        if user.ereader_email.is_empty() {
                            <p class="text-sm text-muted-foreground">
                                (t!(ereader_missing)) " "
                                <a href=(href!(crate::app::account::page)) class="font-medium text-foreground underline underline-offset-4">(t!(nav_account))</a>
                            </p>
                        } else {
                            <form method="post" action=(href!(send, BookId(id))) class="flex flex-wrap items-center gap-2">
                                <select name="format" aria-label=(t!(format_to_send)) class="h-8 rounded-md border border-border bg-transparent px-2 text-sm">
                                    for format in &sendable { <option value=(*format)>(*format)</option> }
                                </select>
                                button(variant: ButtonVariant::Outline, size: ButtonSize::Sm, attrs: attributes! { type="submit" }, (t!(send_to, email = user.ereader_email.as_str())))
                            </form>
                        }
                    </section>
                }

                if can_edit && !book.formats.is_empty() && !targets.is_empty() {
                    <section class="mt-8 border-t border-border pt-6">
                        <h2 class="mb-3 flex items-center gap-2 text-sm font-semibold">
                            icon(data: iconify_icon!("lucide:repeat"))
                            (t!(convert))
                        </h2>
                        <form method="post" action=(href!(convert, BookId(id))) class="flex flex-wrap items-center gap-2 text-sm">
                            <select name="from" aria-label=(t!(convert_from)) class="h-8 rounded-md border border-border bg-transparent px-2">
                                for format in &book.formats { <option value=(&format.format)>(&format.format)</option> }
                            </select>
                            icon(data: iconify_icon!("lucide:arrow-right"), attrs: attributes! { class="text-muted-foreground" })
                            <select name="to" aria-label=(t!(convert_to)) class="h-8 rounded-md border border-border bg-transparent px-2">
                                for format in &targets { <option value=(*format)>(*format)</option> }
                            </select>
                            button(variant: ButtonVariant::Outline, size: ButtonSize::Sm, attrs: attributes! { type="submit" }, (t!(convert)))
                        </form>
                    </section>
                }
            </div>
        </article>
    })
}

#[derive(Deserialize)]
pub struct SendForm {
    format: String,
}

/// Emails one format of the book to the user's e-reader address.
#[route(POST "./send")]
pub async fn send(cx: &Cx, Form(form): Form<SendForm>) -> Result<SeeOther> {
    let user = require(cx, Perm::Download).await?.clone();
    let id = *path_param::<BookId>(cx)?;
    let book = library::book(id, &hidden_tags(cx).await)?.ok_or_not_found()?;
    let back = href!(page, BookId(id)).resolve(cx);
    if user.ereader_email.is_empty() {
        return Ok(see_other(with_error(back, &t!(ereader_missing).resolve(cx))));
    }
    let (file, name) = library::format_file(id, &form.format, &hidden_tags(cx).await)?.ok_or_not_found()?;
    Ok(see_other(
        match mailer::send_book(cx, &user.ereader_email, &book.title, &file, &name, content_type(&form.format)).await {
            Ok(()) => with_msg(back, &t!(sent_to, email = user.ereader_email.as_str()).resolve(cx)),
            Err(error) => with_error(back, &t!(send_failed, error = error.to_string()).resolve(cx)),
        },
    ))
}

#[derive(Deserialize)]
pub struct ConvertForm {
    from: String,
    to: String,
}

/// Starts converting one format of the book into another, in the background.
#[route(POST "./convert")]
pub async fn convert(cx: &Cx, Form(form): Form<ConvertForm>) -> Result<SeeOther> {
    let user_id = require(cx, Perm::Edit).await?.id;
    let id = *path_param::<BookId>(cx)?;
    let book = library::book(id, &hidden_tags(cx).await)?.ok_or_not_found()?;
    let to = OUTPUT_FORMATS
        .into_iter()
        .find(|f| f.eq_ignore_ascii_case(&form.to))
        .ok_or_bad_request("unknown output format")?;
    let (source, _) = library::format_file(id, &form.from, &hidden_tags(cx).await)?.ok_or_not_found()?;
    let from = form.from.to_uppercase();
    let label = t!(convert_task, title = book.title.as_str(), from = from.as_str(), to).resolve(cx).into_owned();
    calibre::spawn_task(label, user_id, async move { calibre::convert(id, &source, to).await });
    Ok(see_other(with_msg(href!(tasks::page).resolve(cx), &t!(conversion_started).resolve(cx))))
}

/// Links to the category pages of a book's authors, publishers...
#[component]
async fn item_links(kind: Kind, items: Vec<Item>) -> Result<impl View> {
    Ok(view! {
        for (i, item) in items.into_iter().enumerate() {
            if i > 0 { ", " }
            <a
                href=(href!(item_page, Category(kind), ItemId(item.id)))
                class="font-medium text-foreground underline-offset-4 hover:underline"
            >
                (kind.display(&item.name))
            </a>
        }
    })
}

/// A toggle for each shelf, rendered on the server again whenever `revision` changes.
#[shard]
pub async fn shelf_buttons(cx: &Cx, book_id: u64, revision: Signal<u32>) -> Result<impl View> {
    let _ = revision.get();
    let shelves = Shelf::list(cx, signed_in(cx).await?.id).await?;
    let on = Shelf::shelves_of(cx, book_id).await?;

    Ok(view! {
        if shelves.is_empty() {
            <p class="text-sm text-muted-foreground">
                (t!(no_shelves)) " "
                <a href=(href!(shelves::page)) class="font-medium text-foreground underline underline-offset-4">(t!(create_one))</a>
            </p>
        } else {
            <div class="flex flex-wrap gap-2">
                #[key(shelf.id)]
                for shelf in shelves {
                    let shelf_id = shelf.id;
                    let on_shelf = on.contains(&shelf_id);
                    button(
                        variant: if on_shelf { ButtonVariant::Primary } else { ButtonVariant::Outline },
                        size: ButtonSize::Sm,
                        attrs: attributes! {
                            aria-pressed=(if on_shelf { "true" } else { "false" })
                            @click=$(async |_e: Event| {
                                toggle_shelf(shelf_id, book_id).await;
                                revision.increment();
                            })
                        },
                        if on_shelf {
                            icon(data: iconify_icon!("lucide:check"))
                        } else {
                            icon(data: iconify_icon!("lucide:plus"))
                        }
                        (&shelf.name)
                    )
                }
            </div>
        }
    })
}

/// Marks the book read or unread for the user, rendered again when `revision` changes.
#[shard]
pub async fn read_button(cx: &Cx, book_id: u64, revision: Signal<u32>) -> Result<impl View> {
    let _ = revision.get();
    let read = ReadMark::book_ids(cx, signed_in(cx).await?.id).await?.contains(&book_id);
    Ok(view! {
        button(
            variant: if read { ButtonVariant::Secondary } else { ButtonVariant::Outline },
            attrs: attributes! {
                class="w-full"
                aria-pressed=(if read { "true" } else { "false" })
                @click=$(async |_e: Event| {
                    toggle_read(book_id).await;
                    revision.increment();
                })
            },
            if read {
                icon(data: iconify_icon!("lucide:book-check"))
                (t!(nav_read))
            } else {
                icon(data: iconify_icon!("lucide:book"))
                (t!(mark_read))
            }
        )
    })
}

#[procedure]
pub async fn toggle_read(cx: &Cx, book_id: u64) -> Result<()> {
    ReadMark::toggle(cx, signed_in(cx).await?.id, book_id).await
}

/// Puts a book on a shelf, or takes it off. Both ids come from the browser, so the
/// shelf is checked to belong to the user.
#[procedure]
pub async fn toggle_shelf(cx: &Cx, shelf_id: u64, book_id: u64) -> Result<()> {
    if Shelf::get(cx, shelf_id, signed_in(cx).await?.id).await?.is_some() {
        Shelf::toggle(cx, shelf_id, book_id).await?;
    }
    Ok(())
}
