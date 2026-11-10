pub mod shelf_id;

use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    router::{href, page},
    runtime::{Event, Signal, procedure, shard, signal},
    view::{View, attributes, view},
};

use super::{hidden_tags, signed_in, ui::cover};
use crate::t;
use crate::{
    components::{
        button::{ButtonSize, ButtonVariant, button},
        input::input,
    },
    features::{
        library::{self, Filter},
        shelf::Shelf,
    },
};
use shelf_id::ShelfId;

/// The shelves, with a form to create one.
#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let name = signal(cx, String::new);
    let revision = signal(cx, || 0u32);

    Ok(view! {
        <h1 class="font-serif text-3xl font-medium md:text-4xl">(t!(nav_shelves))</h1>
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
    let hidden = hidden_tags(cx).await;
    let mut shelves = Vec::new();
    for shelf in Shelf::list(cx, signed_in(cx).await?.id).await? {
        let ids = Shelf::book_ids(cx, shelf.id).await?;
        // The first few books, standing on the shelf as a preview.
        let preview = library::books(&Filter::Ids(&ids), "title", 1, &hidden)?.0.into_iter().take(5).collect::<Vec<_>>();
        shelves.push((shelf, ids.len(), preview));
    }

    Ok(view! {
        if shelves.is_empty() {
            <p class="mt-8 text-muted-foreground">(t!(no_shelves))</p>
        } else {
            <ul class="mt-8 grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
                #[key(shelf.id)]
                for (shelf, count, preview) in shelves {
                    let id = shelf.id;
                    <li class="relative rounded-xl border border-border bg-card p-4 transition-colors has-[a:hover]:border-primary/60 has-[a:focus-visible]:ring-2 has-[a:focus-visible]:ring-ring">
                        // The covers stand on a plank: the border under them.
                        <div class="flex h-24 items-end gap-1.5 border-b-2 border-foreground/15 px-1 pr-10">
                            if preview.is_empty() {
                                <div class="mb-px h-16 w-11 rounded-[3px] border border-dashed border-foreground/20"></div>
                            }
                            #[key(book.id)]
                            for book in preview {
                                <div class="w-12 shrink-0">cover(id: book.id, title: book.title.clone(), has_cover: book.has_cover)</div>
                            }
                        </div>
                        <a href=(href!(shelf_id::page, ShelfId(id))) class="mt-3 block outline-none after:absolute after:inset-0">
                            <span class="block truncate font-serif text-lg font-medium">(&shelf.name)</span>
                            <span class="block text-xs text-muted-foreground">(t!(book_count, count))</span>
                        </a>
                        <div class="absolute top-2 right-2 z-10">
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
                        </div>
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
