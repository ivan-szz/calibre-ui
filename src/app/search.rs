use topcoat::{
    Result,
    context::Cx,
    router::{page, query_params},
    view::{View, view},
};

use super::ui::{ListQuery, book_list};
use crate::{features::library::Filter, t};

/// Books whose title, authors, tags, or series match `?q=`.
#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let q = query_params::<ListQuery>(cx)?.q.clone().unwrap_or_default();
    let title = t!(search_results, query = q.trim()).resolve(cx).into_owned();
    Ok(view! {
        if q.trim().is_empty() {
            <h1 class="font-serif text-3xl font-medium md:text-4xl">(t!(search_title))</h1>
            <p class="mt-2 text-muted-foreground">(t!(search_hint))</p>
        } else {
            book_list(title: title, filter: Filter::Search(&q), default_sort: "title")
        }
    })
}
