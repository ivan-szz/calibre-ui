use topcoat::{
    Result,
    context::Cx,
    router::{error::RouterErrorExt, module_param, page, path_param},
    view::{View, view},
};

use crate::{
    app::{signed_in, ui::book_list},
    features::{library::Filter, shelf::Shelf},
};

module_param!(pub shelf_id: u64, error = not_found);

/// The books on one shelf.
#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let id = *path_param::<ShelfId>(cx)?;
    let shelf = Shelf::get(cx, id, signed_in(cx).await?.id).await?.ok_or_not_found()?;
    let ids = Shelf::book_ids(cx, id).await?;
    Ok(view! {
        book_list(title: shelf.name, filter: Filter::Ids(&ids), default_sort: "title")
    })
}
