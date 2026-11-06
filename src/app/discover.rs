use topcoat::{
    Result,
    icon::{icon, iconify::iconify_icon},
    router::page,
    view::{View, view},
};

use super::ui::book_grid;
use crate::{
    components::button::{ButtonSize, ButtonVariant, button_variants},
    features::library::{self, Filter},
    t,
};

/// A random selection of books.
#[page]
pub async fn page(cx: &topcoat::context::Cx) -> Result<impl View> {
    let (books, _) = library::books(&Filter::All, "random", 1, &super::hidden_tags(cx).await)?;
    Ok(view! {
        <div class="mb-8 flex flex-wrap items-end justify-between gap-4">
            <div>
                <h1 class="text-2xl font-semibold tracking-tight md:text-3xl">(t!(nav_discover))</h1>
                <p class="mt-1 text-sm text-muted-foreground">(t!(discover_subtitle))</p>
            </div>
            <a href="" class=(button_variants(ButtonVariant::Outline, ButtonSize::Sm))>
                icon(data: iconify_icon!("lucide:shuffle"))
                (t!(shuffle))
            </a>
        </div>
        book_grid(books: books)
    })
}
