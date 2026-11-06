//! Sign-in through the OpenID Connect provider in `features::oidc`.

use topcoat::{
    Result,
    context::Cx,
    cookie::{Cookie, Cookies as _, SameSite, cookies, time::Duration},
    router::{
        error::{RedirectError, SeeOther, not_found, redirect, see_other},
        href, query_params, route,
    },
};

use super::page as login_page;
use crate::{
    app::ui::with_error,
    features::{auth, oidc},
    t,
};

/// The cookie tying a callback to the browser that started the sign-in.
const STATE_COOKIE: &str = "oidc_state";

/// Sends the browser to the provider.
#[route(GET)]
pub async fn start(cx: &Cx) -> Result<RedirectError> {
    let config = oidc::config().ok_or_else(not_found)?;
    let state = auth::random_token();
    let url = oidc::authorize_url(&config, &state).await?;
    // Lax lets the cookie come back on the provider's top-level redirect.
    cookies(cx).add(
        Cookie::build((STATE_COOKIE, state))
            .path("/login/oidc")
            .http_only(true)
            .secure(config.public_url.starts_with("https://"))
            .same_site(SameSite::Lax)
            .max_age(Duration::minutes(10)),
    );
    Ok(redirect(url))
}

#[query_params]
struct Callback {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

/// Where the provider sends the browser back with a code to trade for the user.
#[route(GET "./callback")]
pub async fn callback(cx: &Cx) -> Result<SeeOther> {
    let config = oidc::config().ok_or_else(not_found)?;
    let fail = |cx: &Cx| see_other(with_error(href!(login_page).resolve(cx), &t!(oidc_failed).resolve(cx)));
    let Ok(query) = query_params::<Callback>(cx) else { return Ok(fail(cx)) };
    let expected = cookies(cx).get(STATE_COOKIE).map(|c| c.value().to_owned());
    cookies(cx).remove(Cookie::build(STATE_COOKIE).path("/login/oidc"));
    // A mismatched state means the callback was not started here: a forged login.
    let (Some(code), Some(state), Some(expected), None) = (&query.code, &query.state, expected, &query.error) else {
        return Ok(fail(cx));
    };
    if *state != expected {
        return Ok(fail(cx));
    }
    let name = match oidc::username(&config, code).await {
        Ok(name) => name,
        Err(error) => {
            eprintln!("OIDC sign-in failed: {error:#}");
            return Ok(fail(cx));
        }
    };
    let Some(user) = auth::external_user(cx, &name).await? else {
        return Ok(see_other(with_error(href!(login_page).resolve(cx), &t!(oidc_name_taken, name).resolve(cx))));
    };
    auth::start_session(cx, &user).await?;
    Ok(see_other(href!(crate::app::home).resolve(cx)))
}
