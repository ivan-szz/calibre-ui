pub mod user_id;

use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    router::{
        content::Form,
        error::{SeeOther, see_other},
        href, page, route,
    },
    view::{View, attributes, view},
};

use super::PermsForm;
use crate::{
    app::{require, ui::{checkbox, field, heading, with_error, with_msg}},
    components::{
        button::button,
        card::{card, card_content, card_header, card_title},
        input::input,
    },
    features::auth::{Perm, User},
    t,
};
use user_id::UserId;

/// Every account, and a form to create one.
#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    require(cx, Perm::Admin).await?;
    let users = User::list(cx).await?;
    Ok(view! {
        heading(title: t!(nav_users).resolve(cx), subtitle: t!(users_subtitle).resolve(cx))
        <div class="mt-8 grid gap-6 xl:grid-cols-[1fr_22rem]">
            <div class="overflow-hidden rounded-xl border border-border bg-card shadow-sm">
                <table class="w-full text-left text-sm">
                    <thead class="border-b border-border text-xs text-muted-foreground">
                        <tr>
                            <th class="px-4 py-3 font-medium">(t!(username))</th>
                            <th class="px-4 py-3 font-medium">(t!(permissions))</th>
                            <th class="px-4 py-3"><span class="sr-only">(t!(actions))</span></th>
                        </tr>
                    </thead>
                    <tbody>
                        for user in users {
                            <tr class="border-b border-border last:border-0">
                                <td class="px-4 py-3 font-medium">(&user.name)</td>
                                <td class="px-4 py-3">
                                    <span class="flex flex-wrap gap-1">
                                        for (on, label) in [
                                            (user.admin, t!(perm_admin_short)),
                                            (user.can(Perm::Download), t!(perm_download_short)),
                                            (user.can(Perm::Upload), t!(perm_upload_short)),
                                            (user.can(Perm::Edit), t!(perm_edit_short)),
                                        ] {
                                            if on {
                                                <span class="rounded-full bg-foreground/5 px-2 py-0.5 text-xs">(label)</span>
                                            }
                                        }
                                    </span>
                                </td>
                                <td class="px-4 py-3 text-right">
                                    <a href=(href!(user_id::page, UserId(user.id))) class="inline-flex items-center gap-1 text-sm font-medium hover:underline">
                                        (t!(edit)) icon(data: iconify_icon!("lucide:chevron-right"))
                                    </a>
                                </td>
                            </tr>
                        }
                    </tbody>
                </table>
            </div>
            card(
                attrs: attributes! { class="self-start" },
                card_header(card_title((t!(add_user))))
                card_content(
                    <form method="post" action=(href!(create)) class="grid gap-4">
                        field(label: t!(username).resolve(cx), input(attrs: attributes! { name="name" required="" maxlength="64" }))
                        field(label: t!(password).resolve(cx),
                            input(attrs: attributes! { name="password" type="password" required="" autocomplete="new-password" })
                        )
                        <fieldset class="grid gap-2">
                            <legend class="mb-1 text-sm font-medium">(t!(permissions))</legend>
                            checkbox(name: "download", label: t!(perm_download), checked: true)
                            checkbox(name: "upload", label: t!(perm_upload), checked: false)
                            checkbox(name: "edit", label: t!(perm_edit), checked: false)
                            checkbox(name: "admin", label: t!(perm_admin), checked: false)
                        </fieldset>
                        button(attrs: attributes! { type="submit" class="justify-self-start" }, (t!(add_user)))
                    </form>
                )
            )
        </div>
    })
}

#[derive(Deserialize)]
pub struct NewUser {
    name: String,
    password: String,
    #[serde(flatten)]
    perms: PermsForm,
}

#[route(POST "./create")]
pub async fn create(cx: &Cx, Form(form): Form<NewUser>) -> Result<SeeOther> {
    require(cx, Perm::Admin).await?;
    let back = href!(page).resolve(cx);
    let name = form.name.trim();
    if name.is_empty() || name.len() > 64 || form.password.is_empty() {
        return Ok(see_other(with_error(back, &t!(account_rule).resolve(cx))));
    }
    if !User::add(cx, name, &form.password, form.perms.perms()).await? {
        return Ok(see_other(with_error(back, &t!(username_taken).resolve(cx))));
    }
    Ok(see_other(with_msg(back, &t!(user_added, name).resolve(cx))))
}
