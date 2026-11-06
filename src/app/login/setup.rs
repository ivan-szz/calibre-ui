use topcoat::{
    Result,
    context::Cx,
    router::{
        content::Form,
        error::{SeeOther, forbidden, see_other},
        href, route,
    },
};

use super::{Credentials, page};
use crate::{
    app::ui::with_error,
    features::auth::{self, Perms, User},
};

/// Creates the first account, as an admin, and signs it in. Refused once any account
/// exists, since this route is public.
#[route(POST)]
pub async fn setup(cx: &Cx, Form(form): Form<Credentials>) -> Result<SeeOther> {
    if User::any_exist(cx).await? {
        return Err(forbidden().into());
    }
    let name = form.name.trim();
    if name.is_empty() || form.password.is_empty() {
        return Ok(see_other(with_error(href!(page).resolve(cx), &crate::t!(account_rule).resolve(cx))));
    }
    let all = Perms { admin: true, download: true, upload: true, edit: true };
    User::add(cx, name, &form.password, all).await?;
    auth::login(cx, name, &form.password).await?;
    Ok(see_other(href!(crate::app::home).resolve(cx)))
}
