pub mod item_id;

use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    router::{href, module_param, page, path_param},
    view::{View, view},
};

use crate::{app::{kind_icon, kind_label}, features::library::{self, Kind}, t};
use item_id::ItemId;

module_param!(pub category: Kind, error = not_found);

/// Every author, series, tag... of one category, with its book count.
#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let kind = *path_param::<Category>(cx)?;
    let items = library::items(kind, &crate::app::hidden_tags(cx).await)?;
    Ok(view! {
        <h1 class="flex items-center gap-3 text-2xl font-semibold tracking-tight md:text-3xl">
            <span class="text-muted-foreground">icon(data: kind_icon(kind))</span>
            (kind_label(kind))
        </h1>
        <p class="mt-1 mb-8 text-sm text-muted-foreground">(t!(in_library, count = items.len()))</p>
        if items.is_empty() {
            <p class="text-muted-foreground">(t!(nothing_here))</p>
        } else {
            <ul class="grid gap-2 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
                for item in items {
                    <li>
                        <a
                            href=(href!(item_id::page, Category(kind), ItemId(item.id.clone())))
                            class="group flex items-center justify-between gap-3 rounded-lg border border-border bg-card px-4 py-3 shadow-xs transition-colors hover:border-foreground/20 hover:bg-foreground/[0.03]"
                        >
                            <span class="truncate text-sm font-medium">(kind.display(&item.name))</span>
                            <span class="flex shrink-0 items-center gap-1 text-xs text-muted-foreground tabular-nums">
                                (item.count)
                                icon(data: iconify_icon!("lucide:chevron-right"), attrs: topcoat::view::attributes! {
                                    class="transition-transform group-hover:translate-x-0.5"
                                })
                            </span>
                        </a>
                    </li>
                }
            </ul>
        }
    })
}
