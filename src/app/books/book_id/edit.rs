//! Editing a book. Every change is made by Calibre's `calibredb`.

pub mod fetch;

use std::collections::HashMap;

use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    router::{
        content::{Form, multipart::Multipart},
        error::{RouterErrorExt, SeeOther, see_other},
        href, page, path_param, route,
    },
    view::{View, attributes, view},
};

use super::{BookId, page as book_page};
use crate::{
    app::{
        require,
        ui::{field, heading, series_number, with_error, with_msg},
    },
    components::{
        button::{ButtonVariant, button, button_variants, ButtonSize},
        card::{card, card_content, card_description, card_header, card_title},
        input::input,
    },
    features::{
        auth::Perm,
        calibre,
        library::{self, Book},
        shelf::Shelf,
    },
    t,
};
use topcoat::l10n::Translation;

/// The text fields of the form, by their `calibredb` field name.
const FIELDS: [&str; 8] = ["title", "authors", "series", "series_index", "tags", "publisher", "pubdate", "languages"];

/// The label of a metadata field.
pub fn field_label(name: &str) -> Translation<'static> {
    match name {
        "title" => t!(field_title),
        "authors" => t!(kind_authors),
        "series" => t!(kind_series),
        "series_index" => t!(field_series_index),
        "tags" => t!(kind_tags),
        "publisher" => t!(publisher),
        "pubdate" => t!(published),
        "languages" => t!(kind_languages),
        "rating" => t!(rating),
        "comments" => t!(field_description),
        _ => t!(field_cover),
    }
}

/// How to fill in a metadata field, if it needs saying.
fn field_hint(name: &str) -> Option<Translation<'static>> {
    match name {
        "authors" => Some(t!(hint_authors)),
        "tags" => Some(t!(hint_commas)),
        "pubdate" => Some(t!(hint_date)),
        "languages" => Some(t!(hint_languages)),
        _ => None,
    }
}

/// The book's current value of each editable field, as the form shows it.
pub fn current_values(book: &Book) -> HashMap<&'static str, String> {
    let names = |items: &[library::Item], sep: &str| items.iter().map(|i| i.name.clone()).collect::<Vec<_>>().join(sep);
    HashMap::from([
        ("title", book.title.clone()),
        ("authors", names(&book.authors, " & ")),
        ("series", names(&book.series, "")),
        ("series_index", series_number(book.series_index)),
        ("tags", names(&book.tags, ", ")),
        ("publisher", names(&book.publishers, "")),
        ("pubdate", book.published.clone().unwrap_or_default()),
        ("languages", names(&book.languages, ", ")),
        ("rating", book.ratings.first().map_or("0".into(), |r| (r.name.parse::<u32>().unwrap_or(0) / 2).to_string())),
        ("comments", book.description.join("\n\n")),
    ])
}

pub async fn editable_book(cx: &Cx) -> Result<Book> {
    require(cx, Perm::Edit).await?;
    Ok(library::book(*path_param::<BookId>(cx)?, &crate::app::hidden_tags(cx).await)?.ok_or_not_found()?)
}

/// The metadata form, plus format and delete actions.
#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let book = editable_book(cx).await?;
    let id = book.id;
    let values = current_values(&book);
    let rating: u32 = values["rating"].parse().unwrap_or(0);
    let hint = |h: Option<Translation<'static>>| h.map(|h| h.resolve(cx).into_owned()).unwrap_or_default();

    Ok(view! {
        <div class="flex flex-wrap items-end justify-between gap-4">
            heading(title: t!(edit_title, title = book.title.as_str()).resolve(cx), subtitle: t!(edit_subtitle).resolve(cx))
            <a href=(href!(book_page, BookId(id))) class=(button_variants(ButtonVariant::Outline, ButtonSize::Sm))>(t!(back_to_book))</a>
        </div>
        <div class="mt-8 grid gap-6 xl:grid-cols-[1fr_20rem]">
            <form method="post" action=(href!(save, BookId(id))) enctype="multipart/form-data" class="grid gap-5 rounded-xl border border-border bg-card p-6 shadow-sm sm:grid-cols-2">
                for name in FIELDS {
                    field(label: field_label(name).resolve(cx), hint: hint(field_hint(name)),
                        input(attrs: attributes! {
                            name=(name)
                            value=(values[name].clone())
                            required=(name == "title" || name == "authors")
                            type=(if name == "pubdate" { "date" } else if name == "series_index" { "number" } else { "text" })
                            step=((name == "series_index").then_some("any"))
                        })
                    )
                }
                field(label: t!(rating).resolve(cx),
                    <select name="rating" class="h-9 rounded-lg border border-border bg-transparent px-3 text-sm">
                        for stars in 0..=5u32 {
                            <option value=(stars) selected=(stars == rating)>
                                if stars == 0 { (t!(no_rating)) } else { ("★".repeat(stars as usize)) }
                            </option>
                        }
                    </select>
                )
                field(label: t!(field_cover).resolve(cx), hint: t!(hint_cover).resolve(cx),
                    input(attrs: attributes! { name="cover" type="file" accept="image/jpeg,image/png" })
                )
                for custom in book.custom.iter().filter(|f| f.editable()) {
                    field(
                        label: custom.name.clone(),
                        hint: hint(custom_hint(&custom.datatype, custom.is_multiple)),
                        input(attrs: attributes! {
                            name=(format!("custom:{}", custom.label))
                            value=(&custom.value)
                        })
                    )
                }
                <div class="sm:col-span-2">
                    field(label: t!(field_description).resolve(cx), hint: t!(hint_description).resolve(cx),
                        <textarea name="comments" rows="8" class="w-full rounded-lg border border-border bg-transparent px-3 py-2 text-sm">(&values["comments"])</textarea>
                    )
                </div>
                <div class="sm:col-span-2">
                    button(attrs: attributes! { type="submit" }, (t!(save_changes)))
                </div>
            </form>

            <div class="grid content-start gap-6">
                card(
                    card_header(
                        card_title((t!(fetch_metadata)))
                        card_description((t!(fetch_metadata_hint)))
                    )
                    card_content(
                        <a href=(href!(fetch::page, BookId(id))) class=(button_variants(ButtonVariant::Outline, ButtonSize::Sm))>(t!(search_online))</a>
                    )
                )
                card(
                    card_header(
                        card_title((t!(kind_formats)))
                        card_description((t!(formats_hint)))
                    )
                    card_content(
                        if book.formats.is_empty() {
                            <p class="text-sm text-muted-foreground">(t!(no_files))</p>
                        }
                        <ul class="grid gap-2">
                            for format in &book.formats {
                                <li>
                                    <form method="post" action=(href!(remove_format, BookId(id))) class="flex items-center justify-between gap-2 text-sm">
                                        <input type="hidden" name="format" value=(&format.format)>
                                        <span class="font-medium">(&format.format)</span>
                                        button(variant: ButtonVariant::Ghost, size: ButtonSize::Sm, attrs: attributes! { type="submit" }, (t!(remove)))
                                    </form>
                                </li>
                            }
                        </ul>
                    )
                )
                card(
                    card_header(
                        card_title((t!(delete_book)))
                        card_description((t!(delete_book_hint)))
                    )
                    card_content(
                        <details>
                            <summary class=(button_variants(ButtonVariant::Outline, ButtonSize::Sm))>(t!(delete_book_open))</summary>
                            <form method="post" action=(href!(delete, BookId(id))) class="mt-3">
                                button(variant: ButtonVariant::Destructive, size: ButtonSize::Sm, attrs: attributes! { type="submit" }, (t!(delete_book_confirm, title = book.title.as_str())))
                            </form>
                        </details>
                    )
                )
            </div>
        </div>
    })
}

/// How to fill in a custom column of this type.
fn custom_hint(datatype: &str, is_multiple: bool) -> Option<Translation<'static>> {
    match datatype {
        "bool" => Some(t!(hint_yes_no)),
        "rating" => Some(t!(hint_stars)),
        "datetime" => Some(t!(hint_date)),
        "int" | "float" => Some(t!(hint_number)),
        _ if is_multiple => Some(t!(hint_commas)),
        _ => None,
    }
}

/// The custom columns that differ from the book's current values, as
/// `(lookup name, value)` for `calibredb set_custom`.
fn changed_custom(book: &Book, submitted: &HashMap<String, String>) -> Vec<(String, String)> {
    book.custom
        .iter()
        .filter(|f| f.editable())
        .filter_map(|f| {
            let value = submitted.get(&format!("custom:{}", f.label))?.trim().replace("\r\n", "\n");
            (value != f.value).then(|| {
                let value = match f.datatype.as_str() {
                    "comments" => paragraphs_html(&value),
                    // The form has stars; `set_custom` stores half-stars as given.
                    "rating" => value.parse::<u32>().map_or(value, |stars| (stars.min(5) * 2).to_string()),
                    "datetime" => calibre::calibre_date(&value),
                    _ => value,
                };
                (f.label.clone(), value)
            })
        })
        .collect()
}

/// Escapes text for the HTML description Calibre stores.
fn paragraphs_html(text: &str) -> String {
    text.split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|p| format!("<p>{}</p>", p.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('\n', "<br>")))
        .collect()
}

/// The `calibredb` fields that differ from the book's current values.
fn changed_fields(current: &HashMap<&'static str, String>, submitted: &HashMap<String, String>) -> Vec<(&'static str, String)> {
    let mut fields = Vec::new();
    for name in FIELDS.into_iter().chain(["rating", "comments"]) {
        let Some(value) = submitted.get(name) else { continue };
        let value = value.trim().replace("\r\n", "\n");
        if value == current[name] {
            continue;
        }
        // The form's names and list formats are already the ones `calibredb` takes.
        fields.push((name, if name == "comments" { paragraphs_html(&value) } else { value }));
    }
    fields
}

/// Saves the changed fields and the new cover, if any.
#[route(POST "./save")]
pub async fn save(cx: &Cx, mut multipart: Multipart) -> Result<SeeOther> {
    let book = editable_book(cx).await?;
    let back = href!(page, BookId(book.id)).resolve(cx);
    let mut submitted = HashMap::new();
    let mut cover = None;
    while let Some(part) = multipart.next_field().await? {
        let Some(name) = part.name().map(str::to_owned) else { continue };
        if name == "cover" {
            let content_type = part.content_type().map(|m| m.to_string()).unwrap_or_default();
            let bytes = part.bytes().await?;
            if !bytes.is_empty() {
                let ext = match content_type.as_str() {
                    "image/jpeg" => "jpg",
                    "image/png" => "png",
                    _ => return Ok(see_other(with_error(back, &t!(cover_type).resolve(cx)))),
                };
                cover = Some((bytes, ext));
            }
        } else {
            submitted.insert(name, part.text().await?);
        }
    }
    if submitted.get("title").is_none_or(|t| t.trim().is_empty()) || submitted.get("authors").is_none_or(|a| a.trim().is_empty()) {
        return Ok(see_other(with_error(back, &t!(title_author_required).resolve(cx))));
    }

    let fields = changed_fields(&current_values(&book), &submitted);
    if !fields.is_empty() {
        if let Err(error) = calibre::set_metadata(book.id, &fields).await {
            return Ok(see_other(with_error(back, &error.to_string())));
        }
    }
    for (label, value) in changed_custom(&book, &submitted) {
        if let Err(error) = calibre::set_custom(book.id, &label, &value).await {
            return Ok(see_other(with_error(back, &error.to_string())));
        }
    }
    if let Some((bytes, ext)) = cover {
        let dir = calibre::temp_dir()?;
        let path = dir.join(format!("cover.{ext}"));
        tokio::fs::write(&path, &bytes).await?;
        let result = calibre::set_cover(book.id, &path).await;
        let _ = std::fs::remove_dir_all(dir);
        if let Err(error) = result {
            return Ok(see_other(with_error(back, &error.to_string())));
        }
    }
    Ok(see_other(with_msg(href!(book_page, BookId(book.id)).resolve(cx), &t!(changes_saved).resolve(cx))))
}

#[derive(Deserialize)]
pub struct FormatForm {
    format: String,
}

#[route(POST "./remove-format")]
pub async fn remove_format(cx: &Cx, Form(form): Form<FormatForm>) -> Result<SeeOther> {
    let book = editable_book(cx).await?;
    let back = href!(page, BookId(book.id)).resolve(cx);
    // Only a format the book has reaches calibredb.
    let Some(format) = book.formats.iter().find(|f| f.format.eq_ignore_ascii_case(&form.format)) else {
        return Ok(see_other(with_error(back, &t!(no_such_format).resolve(cx))));
    };
    Ok(see_other(match calibre::remove_format(book.id, &format.format).await {
        Ok(()) => with_msg(back, &t!(format_removed, format = format.format.as_str()).resolve(cx)),
        Err(error) => with_error(back, &error.to_string()),
    }))
}

#[route(POST "./delete")]
pub async fn delete(cx: &Cx) -> Result<SeeOther> {
    let book = editable_book(cx).await?;
    if let Err(error) = calibre::remove_book(book.id).await {
        return Ok(see_other(with_error(href!(page, BookId(book.id)).resolve(cx), &error.to_string())));
    }
    Shelf::forget_book(cx, book.id as u64).await?;
    Ok(see_other(with_msg(href!(crate::app::home).resolve(cx), &t!(book_deleted, title = book.title.as_str()).resolve(cx))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sends_only_changed_fields() {
        let current: HashMap<&'static str, String> =
            FIELDS.into_iter().chain(["rating", "comments"]).map(|n| (n, String::new())).collect();
        let mut current = current;
        current.insert("title", "Dune".into());
        current.insert("comments", "Old".into());
        let submitted = HashMap::from([
            ("title".to_owned(), " Dune ".to_owned()),
            ("tags".to_owned(), "sf, classic".to_owned()),
            ("comments".to_owned(), "A <b>desert</b>\r\n\r\nplanet".to_owned()),
        ]);
        assert_eq!(
            changed_fields(&current, &submitted),
            [("tags", "sf, classic".to_owned()), ("comments", "<p>A &lt;b&gt;desert&lt;/b&gt;</p><p>planet</p>".to_owned())]
        );
    }
}
