//! Fetching metadata from Calibre's online sources, reviewing it, and applying the
//! chosen fields.

use std::collections::HashMap;

use topcoat::{
    Result,
    context::Cx,
    router::{
        content::Form,
        error::{SeeOther, see_other},
        href, page, route,
    },
    view::{View, attributes, view},
};

use super::{super::{BookId, page as book_page}, current_values, editable_book, field_label, page as edit_page};
use crate::{
    app::ui::{heading, with_error, with_msg},
    components::button::{ButtonSize, ButtonVariant, button, button_variants},
    features::{calibre, library::strip_html},
    t,
};

/// Looks the book up online and shows what was found next to what the book has.
#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let book = editable_book(cx).await?;
    let current = current_values(&book);
    let isbn = book.identifiers.iter().find(|(kind, _)| kind == "isbn").map(|(_, v)| v.clone());
    let fetched = calibre::fetch_metadata(&book.title, &current["authors"], isbn.as_deref()).await;
    let id = book.id;

    Ok(view! {
        <div class="flex flex-wrap items-end justify-between gap-4">
            heading(title: t!(fetch_title, title = book.title.as_str()).resolve(cx), subtitle: t!(fetch_subtitle).resolve(cx))
            <a href=(href!(edit_page, BookId(id))) class=(button_variants(ButtonVariant::Outline, ButtonSize::Sm))>(t!(back_to_edit))</a>
        </div>
        match fetched {
            Err(error) => <p role="alert" class="mt-8 rounded-lg border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm">(error.to_string())</p>,
            Ok(None) => <p class="mt-8 text-muted-foreground">(t!(fetch_none))</p>,
            Ok(Some(found)) => {
                <form method="post" action=(href!(apply, BookId(id))) class="mt-8 max-w-4xl">
                    <div class="overflow-hidden rounded-xl border border-border bg-card shadow-sm">
                        <table class="w-full text-left text-sm">
                            <thead class="border-b border-border text-xs text-muted-foreground">
                                <tr>
                                    <th class="w-10 px-4 py-3"><span class="sr-only">(t!(use_field))</span></th>
                                    <th class="px-4 py-3 font-medium">(t!(field))</th>
                                    <th class="px-4 py-3 font-medium">(t!(now))</th>
                                    <th class="px-4 py-3 font-medium">(t!(found))</th>
                                </tr>
                            </thead>
                            <tbody>
                                for (field, value) in found.fields {
                                    let now = current.get(field).cloned().unwrap_or_default();
                                    let shown = if field == "comments" { strip_html(&value).join(" ") } else { value.clone() };
                                    let label = field_label(field).resolve(cx).into_owned();
                                    <tr class="border-b border-border align-top last:border-0">
                                        <td class="px-4 py-3">
                                            <input type="hidden" name=(format!("v:{field}")) value=(&value)>
                                            <input
                                                type="checkbox"
                                                name=(format!("use:{field}"))
                                                aria-label=(t!(use_found, field = label.as_str()))
                                                checked=(now.is_empty() || (field != "comments" && now != value))
                                                class="size-4 accent-primary"
                                            >
                                        </td>
                                        <td class="px-4 py-3 font-medium">(&label)</td>
                                        <td class="px-4 py-3 text-muted-foreground"><span class="line-clamp-4">(now)</span></td>
                                        <td class="px-4 py-3"><span class="line-clamp-4">(shown)</span></td>
                                    </tr>
                                }
                                <tr>
                                    <td class="px-4 py-3">
                                        match &found.isbn { Some(isbn) => <input type="hidden" name="isbn" value=(isbn)>, None => "" }
                                        <input type="checkbox" name="use:cover" aria-label=(t!(use_found_cover)) checked=(!book.has_cover) class="size-4 accent-primary">
                                    </td>
                                    <td class="px-4 py-3 font-medium">(t!(field_cover))</td>
                                    <td class="px-4 py-3 text-muted-foreground">if book.has_cover { (t!(has_cover)) } else { (t!(none)) }</td>
                                    <td class="px-4 py-3">(t!(download_found_cover))</td>
                                </tr>
                            </tbody>
                        </table>
                    </div>
                    button(attrs: attributes! { type="submit" class="mt-4" }, (t!(apply_selected)))
                </form>
            },
        }
    })
}

/// The `(field, value)` pairs the form marked to use, limited to known fields.
fn chosen(form: &HashMap<String, String>) -> Vec<(&'static str, String)> {
    ["title", "authors", "publisher", "pubdate", "tags", "languages", "series", "series_index", "rating", "comments"]
        .into_iter()
        .filter(|f| form.contains_key(&format!("use:{f}")))
        .filter_map(|f| Some((f, form.get(&format!("v:{f}"))?.trim().to_owned())))
        .filter(|(_, v)| !v.is_empty())
        .collect()
}

#[route(POST "./apply")]
pub async fn apply(cx: &Cx, Form(form): Form<HashMap<String, String>>) -> Result<SeeOther> {
    let book = editable_book(cx).await?;
    let back = href!(page, BookId(book.id)).resolve(cx);
    let fields = chosen(&form);
    if !fields.is_empty() {
        if let Err(error) = calibre::set_metadata(book.id, &fields).await {
            return Ok(see_other(with_error(back, &error.to_string())));
        }
    }
    if form.contains_key("use:cover") {
        let authors = current_values(&book)["authors"].clone();
        let dir = calibre::temp_dir()?;
        let path = dir.join("cover.jpg");
        let found = calibre::fetch_cover(&book.title, &authors, form.get("isbn").map(String::as_str), &path).await?;
        let result = if found { calibre::set_cover(book.id, &path).await } else { Ok(()) };
        let _ = std::fs::remove_dir_all(dir);
        result?;
    }
    Ok(see_other(with_msg(href!(book_page, BookId(book.id)).resolve(cx), &t!(metadata_applied).resolve(cx))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_only_checked_known_fields() {
        let form = HashMap::from([
            ("use:title".to_owned(), "on".to_owned()),
            ("v:title".to_owned(), " Dune ".to_owned()),
            ("v:tags".to_owned(), "SF".to_owned()),
            ("use:path".to_owned(), "on".to_owned()),
            ("v:path".to_owned(), "/etc".to_owned()),
        ]);
        assert_eq!(chosen(&form), [("title", "Dune".to_owned())]);
    }
}
