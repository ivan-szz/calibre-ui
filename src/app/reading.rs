use topcoat::{
    Result,
    context::Cx,
    router::page,
    view::{View, view},
};

use super::{signed_in, ui::book_list};
use crate::features::{library::Filter, shelf::ReadMark};

/// The books the user has marked as read.
#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let ids = ReadMark::book_ids(cx, signed_in(cx).await?.id).await?;
    Ok(view! {
        book_list(title: crate::t!(nav_read).resolve(cx).into_owned(), filter: Filter::Ids(&ids), default_sort: "title")
    })
}
