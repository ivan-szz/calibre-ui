//! "Sign in with…": OpenID Connect with the authorization code flow, which covers
//! Google, GitHub (through an OIDC bridge), Keycloak, Authentik, and others.
//!
//! Set `OIDC_ISSUER` (the provider URL, which serves
//! `/.well-known/openid-configuration`), `OIDC_CLIENT_ID`, `OIDC_CLIENT_SECRET`, and
//! `PUBLIC_URL` (this app's address as users reach it, for the redirect back).
//! `OIDC_NAME` names the provider on the sign-in button.

use serde::Deserialize;
use topcoat::{Error, Result};

pub struct Config {
    pub issuer: String,
    pub client_id: String,
    pub client_secret: String,
    pub public_url: String,
    pub name: String,
}

/// The provider settings, when OIDC is configured.
pub fn config() -> Option<Config> {
    let var = |name: &str| std::env::var(name).ok().filter(|v| !v.is_empty());
    Some(Config {
        issuer: var("OIDC_ISSUER")?.trim_end_matches('/').to_owned(),
        client_id: var("OIDC_CLIENT_ID")?,
        client_secret: var("OIDC_CLIENT_SECRET")?,
        public_url: var("PUBLIC_URL")?.trim_end_matches('/').to_owned(),
        name: var("OIDC_NAME").unwrap_or_else(|| "SSO".to_owned()),
    })
}

impl Config {
    /// Where the provider sends the user back.
    pub fn redirect_uri(&self) -> String {
        format!("{}/login/oidc/callback", self.public_url)
    }
}

#[derive(Deserialize)]
struct Discovery {
    authorization_endpoint: String,
    token_endpoint: String,
    userinfo_endpoint: String,
}

#[derive(Deserialize)]
struct Tokens {
    access_token: String,
}

#[derive(Deserialize)]
struct UserInfo {
    sub: String,
    preferred_username: Option<String>,
    email: Option<String>,
}

fn http_error(e: impl std::fmt::Display) -> Error {
    Error::msg(format!("the sign-in provider did not answer: {e}"))
}

/// Runs a blocking HTTP exchange off the async threads.
async fn blocking<T: Send + 'static>(job: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> {
    tokio::task::spawn_blocking(job).await.map_err(http_error)?
}

async fn discover(issuer: &str) -> Result<Discovery> {
    let url = format!("{issuer}/.well-known/openid-configuration");
    blocking(move || ureq::get(&url).call().map_err(http_error)?.body_mut().read_json().map_err(http_error)).await
}

/// Percent-encodes a query value.
fn encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// The provider URL that starts a sign-in. `state` comes back to the callback and
/// ties it to this browser.
pub async fn authorize_url(config: &Config, state: &str) -> Result<String> {
    let discovery = discover(&config.issuer).await?;
    let separator = if discovery.authorization_endpoint.contains('?') { '&' } else { '?' };
    Ok(format!(
        "{}{separator}response_type=code&client_id={}&redirect_uri={}&scope={}&state={}",
        discovery.authorization_endpoint,
        encode(&config.client_id),
        encode(&config.redirect_uri()),
        encode("openid profile email"),
        encode(state),
    ))
}

/// Trades the callback's code for the user's name: their preferred username, else
/// their email, else the provider's subject id.
pub async fn username(config: &Config, code: &str) -> Result<String> {
    let discovery = discover(&config.issuer).await?;
    let form = [
        ("grant_type".to_owned(), "authorization_code".to_owned()),
        ("code".to_owned(), code.to_owned()),
        ("redirect_uri".to_owned(), config.redirect_uri()),
        ("client_id".to_owned(), config.client_id.clone()),
        ("client_secret".to_owned(), config.client_secret.clone()),
    ];
    let info: UserInfo = blocking(move || {
        let tokens: Tokens = ureq::post(&discovery.token_endpoint)
            .send_form(form.iter().map(|(k, v)| (k.as_str(), v.as_str())))
            .map_err(http_error)?
            .body_mut()
            .read_json()
            .map_err(http_error)?;
        ureq::get(&discovery.userinfo_endpoint)
            .header("Authorization", &format!("Bearer {}", tokens.access_token))
            .call()
            .map_err(http_error)?
            .body_mut()
            .read_json()
            .map_err(http_error)
    })
    .await?;
    let name = info.preferred_username.or(info.email).unwrap_or(info.sub);
    let name = name.trim().to_owned();
    if name.is_empty() {
        return Err(Error::msg("the sign-in provider sent no user name"));
    }
    Ok(name)
}
