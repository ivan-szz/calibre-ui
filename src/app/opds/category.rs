pub mod item_id;

use topcoat::{
    Result,
    context::Cx,
    router::{href, module_param, path_param, route},
};

use super::{Feed, NAVIGATION, ACQUISITION, feed, nav_entry};
use crate::features::library::{self, Kind};
use item_id::ItemId;

module_param!(pub category: Kind, error = not_found);

/// Every author, series, tag... as links to their books.
#[route(GET)]
pub async fn list(cx: &Cx) -> Result<Feed> {
    let kind = *path_param::<Category>(cx)?;
    let entries: String = library::items(kind, &crate::app::hidden_tags(cx).await)?
        .into_iter()
        .map(|item| {
            let link = href!(item_id::books, Category(kind), ItemId(item.id.clone())).resolve(cx);
            let count = crate::t!(book_count, count = item.count).resolve(cx);
            nav_entry(cx, &format!("{kind}:{}", item.id), &kind.display(&item.name), &count, &link, ACQUISITION)
        })
        .collect();
    Ok(feed(cx, &kind.to_string(), &crate::app::kind_label(kind).resolve(cx), NAVIGATION, "", &entries))
}
