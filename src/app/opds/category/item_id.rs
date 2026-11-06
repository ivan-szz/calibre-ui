use topcoat::{
    Result,
    context::Cx,
    router::{error::RouterErrorExt, module_param, path_param, route},
};

use super::{super::{Feed, book_feed}, Category};
use crate::features::library::{self, Filter, Kind};

module_param!(pub item_id);

/// The books of one author, series, tag...
#[route(GET)]
pub async fn books(cx: &Cx) -> Result<Feed> {
    let kind = *path_param::<Category>(cx)?;
    let id = path_param::<ItemId>(cx);
    let name = library::item_name(kind, id)?.ok_or_not_found()?;
    let sort = if kind == Kind::Series { "series" } else { "title" };
    book_feed(cx, &format!("{kind}:{id}"), &kind.display(&name), Filter::Item(kind, id), sort).await
}
