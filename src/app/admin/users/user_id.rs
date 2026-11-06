use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    router::{
        content::Form,
        error::{RouterErrorExt, SeeOther, see_other},
        href, module_param, page, path_param, route,
    },
    view::{View, attributes, view},
};

use crate::{
    app::{
        admin::PermsForm,
        require,
        ui::{checkbox, field, heading, with_error, with_msg},
    },
    components::{
        button::{ButtonVariant, button},
        card::{card, card_content, card_description, card_header, card_title},
        input::input,
    },
    features::{auth::{Perm, User}, shelf::Shelf},
    t,
};

module_param!(pub user_id: u64, error = not_found);

async fn target(cx: &Cx) -> Result<User> {
    require(cx, Perm::Admin).await?;
    Ok(User::get(cx, *path_param::<UserId>(cx)?).await?.ok_or_not_found()?)
}

/// One account: its permissions, hidden tags, password, and deletion.
#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let user = target(cx).await?;
    let id = user.id;
    Ok(view! {
        heading(title: user.name.clone(), subtitle: t!(edit_account).resolve(cx))
        <div class="mt-8 grid max-w-4xl gap-6 lg:grid-cols-2">
            card(
                card_header(card_title((t!(permissions))))
                card_content(
                    <form method="post" action=(href!(save_perms, UserId(id))) class="grid gap-3">
                        checkbox(name: "download", label: t!(perm_download), checked: user.can_download)
                        checkbox(name: "upload", label: t!(perm_upload), checked: user.can_upload)
                        checkbox(name: "edit", label: t!(perm_edit), checked: user.can_edit)
                        checkbox(name: "admin", label: t!(perm_admin), checked: user.admin)
                        button(attrs: attributes! { type="submit" class="mt-2 justify-self-start" }, (t!(save_permissions)))
                    </form>
                )
            )
            card(
                card_header(
                    card_title((t!(hidden_tags)))
                    card_description((t!(hidden_tags_hint)))
                )
                card_content(
                    <form method="post" action=(href!(save_hidden, UserId(id))) class="grid gap-4">
                        field(label: t!(kind_tags).resolve(cx), hint: t!(hidden_tags_rule).resolve(cx),
                            input(attrs: attributes! { name="tags" value=(&user.hidden_tags) placeholder="Adult, Spoilers" })
                        )
                        button(attrs: attributes! { type="submit" class="justify-self-start" }, (t!(save)))
                    </form>
                )
            )
            card(
                card_header(
                    card_title((t!(reset_password)))
                    card_description((t!(password_change_hint)))
                )
                card_content(
                    <form method="post" action=(href!(reset_password, UserId(id))) class="grid gap-4">
                        field(label: t!(new_password).resolve(cx), hint: t!(password_rule).resolve(cx),
                            input(attrs: attributes! { name="password" type="password" required="" minlength="8" autocomplete="new-password" })
                        )
                        button(attrs: attributes! { type="submit" class="justify-self-start" }, (t!(set_password)))
                    </form>
                )
            )
            card(
                card_header(
                    card_title((t!(delete_account)))
                    card_description((t!(delete_account_hint)))
                )
                card_content(
                    <form method="post" action=(href!(delete, UserId(id)))>
                        button(variant: ButtonVariant::Destructive, attrs: attributes! { type="submit" }, (t!(delete_named, name = user.name.as_str())))
                    </form>
                )
            )
        </div>
    })
}

#[route(POST "./perms")]
pub async fn save_perms(cx: &Cx, Form(form): Form<PermsForm>) -> Result<SeeOther> {
    let user = target(cx).await?;
    let back = href!(page, UserId(user.id)).resolve(cx);
    let mut perms = form.perms();
    // Admins cannot demote themselves, so the library always keeps one.
    if require(cx, Perm::Admin).await?.id == user.id && !perms.admin {
        perms.admin = true;
        User::set_perms(cx, user.id, perms).await?;
        return Ok(see_other(with_error(back, &t!(cannot_demote_self).resolve(cx))));
    }
    User::set_perms(cx, user.id, perms).await?;
    Ok(see_other(with_msg(back, &t!(permissions_saved).resolve(cx))))
}

#[derive(Deserialize)]
pub struct HiddenForm {
    tags: String,
}

#[route(POST "./hidden")]
pub async fn save_hidden(cx: &Cx, Form(form): Form<HiddenForm>) -> Result<SeeOther> {
    let user = target(cx).await?;
    // Stored normalized, so the form shows what is applied.
    let tags: Vec<&str> = form.tags.split(',').map(str::trim).filter(|t| !t.is_empty()).collect();
    User::set_hidden_tags(cx, user.id, &tags.join(", ")).await?;
    Ok(see_other(with_msg(href!(page, UserId(user.id)).resolve(cx), &t!(hidden_tags_saved).resolve(cx))))
}

#[derive(Deserialize)]
pub struct PasswordForm {
    password: String,
}

#[route(POST "./password")]
pub async fn reset_password(cx: &Cx, Form(form): Form<PasswordForm>) -> Result<SeeOther> {
    let user = target(cx).await?;
    let back = href!(page, UserId(user.id)).resolve(cx);
    if form.password.chars().count() < 8 {
        return Ok(see_other(with_error(back, &t!(password_rule).resolve(cx))));
    }
    User::set_password(cx, user.id, &form.password).await?;
    Ok(see_other(with_msg(back, &t!(password_set).resolve(cx))))
}

#[route(POST "./delete")]
pub async fn delete(cx: &Cx) -> Result<SeeOther> {
    let user = target(cx).await?;
    if require(cx, Perm::Admin).await?.id == user.id {
        return Ok(see_other(with_error(href!(page, UserId(user.id)).resolve(cx), &t!(cannot_delete_self).resolve(cx))));
    }
    for shelf in Shelf::list(cx, user.id).await? {
        Shelf::remove(cx, shelf.id).await?;
    }
    User::remove(cx, user.id).await?;
    Ok(see_other(with_msg(href!(super::page).resolve(cx), &t!(deleted_named, name = user.name.as_str()).resolve(cx))))
}
