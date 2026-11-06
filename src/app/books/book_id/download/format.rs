use topcoat::{
    Result,
    context::Cx,
    router::{error::RouterErrorExt, header, module_param, path_param, route},
};

use super::super::BookId;
use crate::{
    app::{require, ui::encode},
    features::{auth::Perm, library},
};

module_param!(pub format);

/// The query string of a download: `?inline=1` shows the file in the browser, for the
/// PDF reader.
#[topcoat::router::query_params(error = bad_request)]
struct DownloadQuery {
    inline: Option<u8>,
}

/// The file of one format of the book, as an attachment.
#[route(GET)]
pub async fn download(cx: &Cx) -> Result<([(header::HeaderName, String); 2], Vec<u8>)> {
    require(cx, Perm::Download).await?;
    let format = path_param::<Format>(cx);
    let hidden = crate::app::hidden_tags(cx).await;
    let (file, name) =
        library::format_file(*path_param::<BookId>(cx)?, format, &hidden)?.ok_or_not_found()?;
    // shortcut: reads the whole file into memory; stream it if libraries hold huge PDFs.
    let bytes = tokio::fs::read(file).await.ok().ok_or_not_found()?;
    let ascii: String = name
        .chars()
        .map(|c| if c.is_ascii_graphic() && c != '"' && c != '\\' || c == ' ' { c } else { '_' })
        .collect();
    let inline = topcoat::router::query_params::<DownloadQuery>(cx)?.inline.is_some();
    let disposition = if inline { "inline" } else { "attachment" };
    Ok((
        [
            (header::CONTENT_TYPE, library::content_type(format).to_owned()),
            (
                header::CONTENT_DISPOSITION,
                format!("{disposition}; filename=\"{ascii}\"; filename*=UTF-8''{}", encode(&name)),
            ),
        ],
        bytes,
    ))
}
