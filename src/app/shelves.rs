pub mod shelf_id;

use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    router::{href, page},
    runtime::{Event, Signal, procedure, shard, signal},
    view::{View, attributes, view},
};

use super::signed_in;
use crate::t;
use crate::{
    components::{
        button::{ButtonSize, ButtonVariant, button},
        input::input,
    },
    features::shelf::Shelf,
};
use shelf_id::ShelfId;

/// The shelves, with a form to create one.
#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let name = signal(cx, String::new);
    let revision = signal(cx, || 0u32);

    Ok(view! {
        <h1 class="text-2xl font-semibold tracking-tight md:text-3xl">(t!(nav_shelves))</h1>
        <p class="mt-1 text-sm text-muted-foreground">(t!(shelves_subtitle))</p>
        <form
            class="mt-6 flex max-w-md gap-2"
            @submit=$(async |e: Event| {
                e.prevent_default();
                create(name.get()).await;
                name.set("".to_owned());
                revision.increment();
            })
        >
            input(attrs: attributes! {
                type="text"
                placeholder=(t!(new_shelf_name))
                aria-label=(t!(new_shelf_name))
                required=""
                maxlength="100"
                :value=$(name.get())
                @input=$(|e: Event| name.set(e.target.value))
            })
            button(attrs: attributes! { type="submit" }, icon(data: iconify_icon!("lucide:plus")) (t!(create)))
        </form>
        shelf_list(revision: revision)
    })
}

/// The shelves, rendered on the server again whenever `revision` changes.
#[shard]
pub async fn shelf_list(cx: &Cx, revision: Signal<u32>) -> Result<impl View> {
    let _ = revision.get();
    let mut shelves = Vec::new();
    for shelf in Shelf::list(cx, signed_in(cx).await?.id).await? {
        let count = Shelf::book_ids(cx, shelf.id).await?.len();
        shelves.push((shelf, count));
    }

    Ok(view! {
        if shelves.is_empty() {
            <p class="mt-8 text-muted-foreground">(t!(no_shelves))</p>
        } else {
            <ul class="mt-8 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
                #[key(shelf.id)]
                for (shelf, count) in shelves {
                    let id = shelf.id;
                    <li class="flex items-center gap-2 rounded-xl border border-border bg-card p-2 pl-4 shadow-xs">
                        <a href=(href!(shelf_id::page, ShelfId(id))) class="flex min-w-0 flex-1 items-center gap-3 py-1">
                            <span class="grid size-9 shrink-0 place-items-center rounded-lg bg-foreground/5">
                                icon(data: iconify_icon!("lucide:bookmark"))
                            </span>
                            <span class="min-w-0">
                                <span class="block truncate text-sm font-medium">(&shelf.name)</span>
                                <span class="block text-xs text-muted-foreground">
                                    (t!(book_count, count))
                                </span>
                            </span>
                        </a>
                        button(
                            variant: ButtonVariant::Ghost,
                            size: ButtonSize::Icon,
                            attrs: attributes! {
                                aria-label=(t!(delete_shelf_named, name = shelf.name.as_str()))
                                title=(t!(delete_shelf))
                                @click=$(async |_e: Event| {
                                    remove(id).await;
                                    revision.increment();
                                })
                            },
                            icon(data: iconify_icon!("lucide:trash-2"))
                        )
                    </li>
                }
            </ul>
        }
    })
}

/// Creates a shelf unless its name is blank or too long.
#[procedure]
pub async fn create(cx: &Cx, name: String) -> Result<()> {
    let name = name.trim();
    if !name.is_empty() && name.chars().count() <= 100 {
        Shelf::add(cx, name, signed_in(cx).await?.id).await?;
    }
    Ok(())
}

/// Deletes one of the user's shelves. Its books stay in the library.
#[procedure]
pub async fn remove(cx: &Cx, id: u64) -> Result<()> {
    if Shelf::get(cx, id, signed_in(cx).await?.id).await?.is_some() {
        Shelf::remove(cx, id).await?;
    }
    Ok(())
}
