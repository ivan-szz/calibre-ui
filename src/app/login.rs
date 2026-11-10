pub mod logout;
pub mod oidc;
pub mod setup;

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

use super::ui::{field, with_error};
use crate::{
    components::{
        button::{ButtonSize, ButtonVariant, button, button_variants},
        card::{card, card_content, card_description, card_header, card_title},
        input::input,
    },
    features::auth::{self, User},
    t,
};

/// The sign-in form, or the form creating the first admin account.
#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let first_run = !User::any_exist(cx).await?;
    let provider = crate::features::oidc::config().map(|c| c.name);
    Ok(view! {
        card(
            attrs: attributes! { class="w-full max-w-sm" },
            card_header(
                <span class="mb-2 text-primary">
                    icon(data: iconify_icon!("lucide:library-big"), size: topcoat::view::Length::px(28.0))
                </span>
                card_title(if first_run { (t!(setup_title)) } else { (t!(login_title)) })
                card_description(if first_run { (t!(setup_hint)) } else { (t!(login_hint)) })
            )
            card_content(
                <form
                    method="post"
                    action=(if first_run { href!(setup::setup).resolve(cx) } else { href!(submit).resolve(cx) })
                    class="grid gap-4"
                >
                    field(label: t!(username).resolve(cx),
                        input(attrs: attributes! { name="name" required="" autocomplete="username" autofocus="" })
                    )
                    field(label: t!(password).resolve(cx),
                        input(attrs: attributes! {
                            name="password"
                            type="password"
                            required=""
                            autocomplete=(if first_run { "new-password" } else { "current-password" })
                        })
                    )
                    button(attrs: attributes! { type="submit" class="mt-2 w-full" },
                        if first_run { (t!(setup_submit)) } else { (t!(login_submit)) }
                    )
                </form>
                match provider.filter(|_| !first_run) {
                    Some(name) => {
                        <div class="my-4 flex items-center gap-3 text-xs text-muted-foreground">
                            <span class="h-px flex-1 bg-border"></span>(t!(or))<span class="h-px flex-1 bg-border"></span>
                        </div>
                        <a href=(href!(oidc::start)) class=(button_variants(ButtonVariant::Outline, ButtonSize::Md)) style="width: 100%">
                            icon(data: iconify_icon!("lucide:key-round"))
                            (t!(sign_in_with, name))
                        </a>
                    },
                    None => "",
                }
            )
        )
    })
}

#[derive(Deserialize)]
pub struct Credentials {
    pub name: String,
    pub password: String,
}

/// Checks the credentials and starts a session.
#[route(POST)]
pub async fn submit(cx: &Cx, Form(form): Form<Credentials>) -> Result<SeeOther> {
    if auth::login(cx, form.name.trim(), &form.password).await? {
        Ok(see_other(href!(crate::app::home).resolve(cx)))
    } else {
        Ok(see_other(with_error(href!(page).resolve(cx), &t!(login_wrong).resolve(cx))))
    }
}
