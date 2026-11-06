use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    router::{
        content::Form,
        error::{SeeOther, see_other},
        href, page, route,
    },
    view::{View, attributes, view},
};

use super::{signed_in, ui::{field, heading, with_error, with_msg}};
use crate::{
    components::{
        button::{ButtonSize, ButtonVariant, button},
        card::{card, card_content, card_description, card_header, card_title},
        input::input,
    },
    features::auth::{self, User},
    t,
};

/// The signed-in user's settings.
#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let user = signed_in(cx).await?.clone();
    Ok(view! {
        heading(title: t!(nav_account).resolve(cx), subtitle: t!(signed_in_as, name = user.name.as_str()).resolve(cx))
        <div class="mt-8 grid max-w-3xl gap-6 lg:grid-cols-2">
            card(
                card_header(
                    card_title((t!(send_to_ereader)))
                    card_description((t!(ereader_hint)))
                )
                card_content(
                    <form method="post" action=(href!(save_ereader)) class="grid gap-4">
                        field(label: t!(ereader_email).resolve(cx),
                            input(attrs: attributes! { name="email" type="email" value=(&user.ereader_email) placeholder="name@kindle.com" })
                        )
                        button(attrs: attributes! { type="submit" class="justify-self-start" }, (t!(save)))
                    </form>
                )
            )
            card(
                card_header(
                    card_title((t!(password)))
                    card_description((t!(password_change_hint)))
                )
                card_content(
                    <form method="post" action=(href!(change_password)) class="grid gap-4">
                        field(label: t!(current_password).resolve(cx),
                            input(attrs: attributes! { name="current" type="password" required="" autocomplete="current-password" })
                        )
                        field(label: t!(new_password).resolve(cx),
                            input(attrs: attributes! { name="new" type="password" required="" autocomplete="new-password" })
                        )
                        button(attrs: attributes! { type="submit" class="justify-self-start" }, (t!(change_password)))
                    </form>
                )
            )
            card(
                attrs: attributes! { class="lg:col-span-2" },
                card_header(
                    card_title((t!(kobo_sync)))
                    card_description((t!(kobo_hint)))
                )
                card_content(
                    if user.kobo_token.is_empty() {
                        <form method="post" action=(href!(kobo))>
                            <input type="hidden" name="action" value="new">
                            button(attrs: attributes! { type="submit" }, (t!(kobo_enable)))
                        </form>
                    } else {
                        <p class="text-sm">(t!(kobo_setting))</p>
                        <code class="mt-2 block rounded-lg bg-foreground/5 px-3 py-2 text-xs break-all">
                            "api_endpoint=" (kobo_url(cx, &user.kobo_token))
                        </code>
                        <div class="mt-4 flex flex-wrap gap-2">
                            <form method="post" action=(href!(kobo))>
                                <input type="hidden" name="action" value="new">
                                button(variant: ButtonVariant::Outline, size: ButtonSize::Sm, attrs: attributes! { type="submit" }, (t!(kobo_new_link)))
                            </form>
                            <form method="post" action=(href!(kobo))>
                                <input type="hidden" name="action" value="off">
                                button(variant: ButtonVariant::Ghost, size: ButtonSize::Sm, attrs: attributes! { type="submit" }, (t!(kobo_disable)))
                            </form>
                        </div>
                    }
                )
            )
        </div>
    })
}

/// The URL a Kobo device syncs with, for its `eReader.conf`.
fn kobo_url(cx: &Cx, token: &str) -> String {
    let origin = std::env::var("PUBLIC_URL").ok().filter(|u| !u.is_empty()).unwrap_or_else(|| {
        let host = topcoat::router::request::headers(cx).get("host").and_then(|h| h.to_str().ok()).unwrap_or("localhost");
        format!("http://{host}")
    });
    format!("{}/kobo/{token}", origin.trim_end_matches('/'))
}

#[derive(Deserialize)]
pub struct KoboForm {
    action: String,
}

/// Turns Kobo sync on with a new secret link, or off.
#[route(POST "./kobo")]
pub async fn kobo(cx: &Cx, Form(form): Form<KoboForm>) -> Result<SeeOther> {
    let user = signed_in(cx).await?;
    let token = (form.action == "new").then(auth::random_token);
    User::set_kobo_token(cx, user.id, token.as_deref()).await?;
    let msg = if token.is_some() { t!(kobo_enabled) } else { t!(kobo_disabled) };
    Ok(see_other(with_msg(href!(page).resolve(cx), &msg.resolve(cx))))
}

#[derive(Deserialize)]
pub struct EreaderForm {
    email: String,
}

#[route(POST "./ereader")]
pub async fn save_ereader(cx: &Cx, Form(form): Form<EreaderForm>) -> Result<SeeOther> {
    let user = signed_in(cx).await?;
    let email = form.email.trim();
    let back = href!(page).resolve(cx);
    if !email.is_empty() && !is_email(email) {
        return Ok(see_other(with_error(back, &t!(not_an_email).resolve(cx))));
    }
    User::set_ereader_email(cx, user.id, email).await?;
    Ok(see_other(with_msg(back, &t!(ereader_saved).resolve(cx))))
}

/// A loose check that catches typos; the mail server has the final say.
pub fn is_email(s: &str) -> bool {
    s.split_once('@').is_some_and(|(local, domain)| {
        !local.is_empty() && domain.contains('.') && !s.contains(char::is_whitespace) && s.len() <= 254
    })
}

#[derive(Deserialize)]
pub struct PasswordForm {
    current: String,
    new: String,
}

#[route(POST "./password")]
pub async fn change_password(cx: &Cx, Form(form): Form<PasswordForm>) -> Result<SeeOther> {
    let user = signed_in(cx).await?.clone();
    let back = href!(page).resolve(cx);
    if form.new.is_empty() {
        return Ok(see_other(with_error(back, &t!(password_rule).resolve(cx))));
    }
    if auth::check_password(cx, &user.name, &form.current).await?.is_none() {
        return Ok(see_other(with_error(back, &t!(current_password_wrong).resolve(cx))));
    }
    User::set_password(cx, user.id, &form.new).await?;
    auth::logout(cx).await?;
    Ok(see_other(with_msg(href!(super::login::page).resolve(cx), &t!(password_changed).resolve(cx))))
}

#[cfg(test)]
mod tests {
    #[test]
    fn checks_emails_loosely() {
        assert!(super::is_email("me@kindle.com"));
        assert!(!super::is_email("me@kindle"));
        assert!(!super::is_email("@kindle.com"));
        assert!(!super::is_email("m e@kindle.com"));
    }
}
