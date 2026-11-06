use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    router::{
        content::multipart::Multipart,
        error::{SeeOther, see_other},
        href, page, route,
    },
    view::{View, attributes, view},
};

use super::{
    books::book_id::{BookId, page as book_page},
    require,
    ui::{heading, with_error, with_msg},
};
use crate::{
    components::button::button,
    features::{auth::Perm, calibre},
    t,
};

/// File extensions Calibre can import.
const ACCEPTED: &str = ".epub,.kepub,.pdf,.mobi,.azw,.azw3,.fb2,.cbz,.cbr,.cb7,.djvu,.docx,.odt,.rtf,.txt,.html,.lit,.prc";

/// The upload form.
#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    require(cx, Perm::Upload).await?;
    Ok(view! {
        heading(title: t!(nav_upload).resolve(cx), subtitle: t!(upload_subtitle).resolve(cx))
        <form method="post" action=(href!(upload)) enctype="multipart/form-data" class="mt-8 max-w-xl">
            <label class="flex cursor-pointer flex-col items-center gap-3 rounded-xl border-2 border-dashed border-border bg-card px-6 py-14 text-center transition-colors hover:border-primary/60 hover:bg-primary/5">
                <span class="grid size-12 place-items-center rounded-full bg-primary/10 text-primary">
                    icon(data: iconify_icon!("lucide:upload"), size: topcoat::view::Length::px(22.0))
                </span>
                <span class="font-medium">(t!(upload_choose))</span>
                <span class="text-sm text-muted-foreground">(t!(upload_formats))</span>
                <input type="file" name="files" multiple="" required="" accept=(ACCEPTED) class="mt-2 text-sm file:mr-3 file:rounded-md file:border-0 file:bg-primary file:px-3 file:py-1.5 file:text-primary-foreground">
            </label>
            button(attrs: attributes! { type="submit" class="mt-4" }, icon(data: iconify_icon!("lucide:upload")) (t!(upload_submit)))
        </form>
    })
}

/// Keeps a file name's extension if Calibre can import it.
fn extension(file_name: &str) -> Option<String> {
    let ext = file_name.rsplit_once('.')?.1.to_ascii_lowercase();
    ACCEPTED.split(',').any(|a| a == format!(".{ext}")).then_some(ext)
}

/// Adds each uploaded file to the library with `calibredb add`.
#[route(POST)]
pub async fn upload(cx: &Cx, mut multipart: Multipart) -> Result<SeeOther> {
    require(cx, Perm::Upload).await?;
    let back = href!(page).resolve(cx);
    let dir = calibre::temp_dir()?;
    let mut added = Vec::new();
    let mut errors = Vec::new();
    while let Some(part) = multipart.next_field().await? {
        let Some(name) = part.file_name().map(str::to_owned) else { continue };
        let Some(ext) = extension(&name) else {
            errors.push(format!("{name}: {}", t!(upload_not_ebook).resolve(cx)));
            continue;
        };
        let bytes = part.bytes().await?;
        if bytes.is_empty() {
            continue;
        }
        // The client's file name is untrusted, so only its checked extension is kept.
        let path = dir.join(format!("upload-{}.{ext}", added.len() + errors.len()));
        tokio::fs::write(&path, &bytes).await?;
        match calibre::add_book(&path).await {
            Ok(Some(id)) => added.push(id),
            Ok(None) => errors.push(format!("{name}: {}", t!(upload_duplicate).resolve(cx))),
            Err(error) => errors.push(format!("{name}: {error}")),
        }
    }
    let _ = std::fs::remove_dir_all(&dir);

    let (done, tried) = (added.len(), added.len() + errors.len());
    Ok(see_other(match (added.as_slice(), errors.is_empty()) {
        ([id], true) => with_msg(href!(book_page, BookId(*id)).resolve(cx), &t!(upload_added_one).resolve(cx)),
        (_, true) if !added.is_empty() => with_msg(href!(crate::app::home).resolve(cx), &t!(upload_added, count = done).resolve(cx)),
        (_, true) => with_error(back, &t!(upload_none).resolve(cx)),
        _ => with_error(back, &format!("{} {}", t!(upload_partial, done, tried).resolve(cx), errors.join("; "))),
    }))
}

#[cfg(test)]
mod tests {
    #[test]
    fn keeps_only_known_extensions() {
        assert_eq!(super::extension("Dune.EPUB").as_deref(), Some("epub"));
        assert_eq!(super::extension("../../evil.sh"), None);
        assert_eq!(super::extension("noext"), None);
    }
}
