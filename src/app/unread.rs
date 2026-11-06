use topcoat::{
    Result,
    context::Cx,
    router::page,
    view::{View, view},
};

use super::{signed_in, ui::book_list};
use crate::features::{library::Filter, shelf::ReadMark};

/// The books the user has not marked as read.
#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let ids = ReadMark::book_ids(cx, signed_in(cx).await?.id).await?;
    Ok(view! {
        book_list(title: crate::t!(nav_unread).resolve(cx).into_owned(), filter: Filter::NotIds(&ids), default_sort: "new")
    })
}
