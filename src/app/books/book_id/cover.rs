use topcoat::{
    Result,
    context::Cx,
    router::{error::RouterErrorExt, header, path_param, route},
};

use super::BookId;
use crate::features::library;

/// The book's cover image.
#[route(GET)]
pub async fn cover(cx: &Cx) -> Result<([(header::HeaderName, &'static str); 2], Vec<u8>)> {
    let hidden = crate::app::hidden_tags(cx).await;
    let file = library::cover_file(*path_param::<BookId>(cx)?, &hidden)?.ok_or_not_found()?;
    let bytes = tokio::fs::read(file).await.ok().ok_or_not_found()?;
    Ok((
        [(header::CONTENT_TYPE, "image/jpeg"), (header::CACHE_CONTROL, "public, max-age=3600")],
        bytes,
    ))
}
