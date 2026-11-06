use topcoat::{
    Result,
    context::Cx,
    router::{error::RouterErrorExt, module_param, page, path_param},
    view::{View, view},
};

use super::Category;
use crate::{
    app::ui::book_list,
    features::library::{self, Filter, Kind},
};

module_param!(pub item_id);

/// The books of one author, series, tag...
#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let kind = *path_param::<Category>(cx)?;
    let id = path_param::<ItemId>(cx);
    let name = library::item_name(kind, id)?.ok_or_not_found()?;
    // A series reads best in its own order.
    let default_sort = if kind == Kind::Series { "series" } else { "new" };
    Ok(view! {
        book_list(title: kind.display(&name), filter: Filter::Item(kind, id), default_sort: default_sort)
    })
}
