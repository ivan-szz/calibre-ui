//! User accounts, their permissions, and login sessions.
//!
//! Browsers authenticate with a session cookie. E-reader apps reading the OPDS feed
//! cannot hold a session, so HTTP Basic credentials are accepted too. Passwords are
//! checked locally, then against LDAP when it is configured; OpenID Connect users sign
//! in through their provider. Accounts from LDAP or OIDC have no local password.

use std::{
    sync::LazyLock,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use topcoat::{
    Result,
    context::{Cx, memoize},
    router::request::headers,
    session::{self, TokenHash},
};

use crate::db::db;

/// An account. An admin has every permission.
#[derive(Clone, Debug, toasty::Model)]
pub struct User {
    #[key]
    #[auto]
    pub id: u64,

    #[unique]
    pub name: String,

    pub password_hash: String,

    /// Where "Send to e-reader" delivers books, such as a Kindle address.
    pub ereader_email: String,

    /// Comma-separated tags whose books this user cannot see, set by an admin.
    pub hidden_tags: String,

    /// The secret in this user's Kobo sync URL, or empty when Kobo sync is off.
    #[index]
    pub kobo_token: String,

    pub admin: bool,
    pub can_download: bool,
    pub can_upload: bool,
    pub can_edit: bool,
}

/// A login session. Only the hash of the browser's token is stored.
#[derive(Debug, toasty::Model)]
pub struct UserSession {
    #[key]
    #[auto]
    pub id: u64,

    #[unique]
    pub token_hash: String,

    #[index]
    pub user_id: u64,

    /// Unix seconds.
    pub expires_at: u64,
}

/// Something a user may be allowed to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Perm {
    Download,
    Upload,
    Edit,
    Admin,
}

impl User {
    /// The tags whose books this user cannot see.
    pub fn hidden(&self) -> Vec<String> {
        self.hidden_tags.split(',').map(str::trim).filter(|t| !t.is_empty()).map(str::to_owned).collect()
    }

    pub fn can(&self, perm: Perm) -> bool {
        self.admin
            || match perm {
                Perm::Download => self.can_download,
                Perm::Upload => self.can_upload,
                Perm::Edit => self.can_edit,
                Perm::Admin => false,
            }
    }
}

/// The permissions given when an account is created or edited.
#[derive(Clone, Copy, Debug, Default)]
pub struct Perms {
    pub admin: bool,
    pub download: bool,
    pub upload: bool,
    pub edit: bool,
}

fn unix_secs(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

fn now() -> u64 {
    unix_secs(SystemTime::now())
}

fn hex(hash: &TokenHash) -> String {
    hash.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Ok(Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| topcoat::Error::msg(e.to_string()))?
        .to_string())
}

fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .is_ok_and(|hash| Argon2::default().verify_password(password.as_bytes(), &hash).is_ok())
}

/// The signed-in user, from the session cookie or HTTP Basic credentials.
pub async fn current_user(cx: &Cx) -> Option<&User> {
    lookup_login(cx).await.as_ref().map(|login| &login.user)
}

/// Who is making the request, and when the session saying so ends.
#[derive(Clone, Debug)]
struct Login {
    user: User,
    /// Unix seconds. `None` for HTTP Basic credentials, which have no session.
    session_expires_at: Option<u64>,
}

#[memoize]
async fn lookup_login(cx: &Cx) -> Option<Login> {
    match resolve_login(cx).await {
        Ok(login) => login,
        Err(error) => {
            eprintln!("resolving the current user failed: {error:#}");
            None
        }
    }
}

async fn resolve_login(cx: &Cx) -> Result<Option<Login>> {
    let mut db = db(cx);
    if let Some(hash) = session::token_hash(cx).await? {
        let record = UserSession::filter_by_token_hash(hex(&hash)).first().exec(&mut db).await?;
        if let Some(record) = record.filter(|s| s.expires_at > now()) {
            let user = User::filter_by_id(record.user_id).first().exec(&mut db).await?;
            return Ok(user.map(|user| Login { user, session_expires_at: Some(record.expires_at) }));
        }
    }
    if let Some((name, password)) = basic_credentials(cx) {
        let user = authenticate(cx, &name, &password).await?;
        return Ok(user.map(|user| Login { user, session_expires_at: None }));
    }
    Ok(None)
}

/// How much of a session's lifetime passes before a visit pushes its expiry forward
/// again, so that a page full of covers does not write the session on every request.
const EXTEND_AFTER: u64 = 60 * 60;

/// Whether a session ending at `expires_at` has gone long enough without being
/// extended to be extended now.
fn due_for_extension(expires_at: u64, now: u64, lifetime: u64) -> bool {
    expires_at.saturating_sub(now) + EXTEND_AFTER.min(lifetime / 2) <= lifetime
}

/// Pushes the session's expiry forward for a signed-in user who is using the app, so
/// the session ends `SESSION_DAYS` after the last visit rather than after signing in.
///
/// This sets a cookie, so call it before the response starts.
pub async fn extend_session(cx: &Cx) -> Result<()> {
    let Some(expires_at) = lookup_login(cx).await.as_ref().and_then(|login| login.session_expires_at) else {
        return Ok(());
    };
    if !due_for_extension(expires_at, now(), session_lifetime().as_secs()) {
        return Ok(());
    }
    let Some(session) = session::refresh(cx).await? else {
        return Ok(());
    };
    let mut db = db(cx);
    if let Some(mut record) = UserSession::filter_by_token_hash(hex(&session.token_hash)).first().exec(&mut db).await? {
        record.update().expires_at(unix_secs(session.expires_at)).exec(&mut db).await?;
    }
    Ok(())
}

/// The user with these credentials, if they are right.
pub async fn check_password(cx: &Cx, name: &str, password: &str) -> Result<Option<User>> {
    let user = User::filter_by_name(name.to_owned()).first().exec(&mut db(cx)).await?;
    Ok(user.filter(|u| verify_password(password, &u.password_hash)))
}

/// The `Authorization: Basic` name and password of the request, if any.
fn basic_credentials(cx: &Cx) -> Option<(String, String)> {
    let value = headers(cx).get("authorization")?.to_str().ok()?.strip_prefix("Basic ")?;
    parse_basic(value)
}

fn parse_basic(value: &str) -> Option<(String, String)> {
    use base64::Engine as _;
    let decoded = base64::engine::general_purpose::STANDARD.decode(value.trim()).ok()?;
    let (name, password) = std::str::from_utf8(&decoded).ok()?.split_once(':')?;
    Some((name.to_owned(), password.to_owned()))
}

/// The marker stored instead of a password hash for LDAP and OIDC accounts. It is not
/// a valid hash, so no local password matches it.
const EXTERNAL: &str = "!external";

/// A random secret, as hex.
pub fn random_token() -> String {
    use argon2::password_hash::rand_core::RngCore as _;
    let mut bytes = [0u8; 24];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The account for a user who signed in through LDAP or OIDC, created on first sign-in
/// with the download permission, as calibre-web does. `None` when the name belongs to
/// a local account: an outside directory must not take over, say, the local `admin`.
pub async fn external_user(cx: &Cx, name: &str) -> Result<Option<User>> {
    let mut db = db(cx);
    if let Some(user) = User::filter_by_name(name.to_owned()).first().exec(&mut db).await? {
        return Ok((user.password_hash == EXTERNAL).then_some(user));
    }
    Ok(Some(toasty::create!(User {
        name,
        password_hash: EXTERNAL,
        ereader_email: "",
        hidden_tags: "",
        kobo_token: "",
        admin: false,
        can_download: true,
        can_upload: false,
        can_edit: false,
    })
    .exec(&mut db)
    .await?))
}

/// The user with these credentials: a local account by its password, otherwise an
/// LDAP account when LDAP is configured and accepts them.
pub async fn authenticate(cx: &Cx, name: &str, password: &str) -> Result<Option<User>> {
    if let Some(user) = check_password(cx, name, password).await? {
        return Ok(Some(user));
    }
    if super::ldap::bind(name, password).await? {
        return external_user(cx, name).await;
    }
    Ok(None)
}

/// Signs in with a name and password. Returns whether they were right.
pub async fn login(cx: &Cx, name: &str, password: &str) -> Result<bool> {
    let Some(user) = authenticate(cx, name, password).await? else {
        return Ok(false);
    };
    start_session(cx, &user).await?;
    Ok(true)
}

/// Starts a session for a user whose identity is already established.
pub async fn start_session(cx: &Cx, user: &User) -> Result<()> {
    let session = session::start(cx).await?;
    toasty::create!(UserSession {
        token_hash: hex(&session.token_hash),
        user_id: user.id,
        expires_at: unix_secs(session.expires_at),
    })
    .exec(&mut db(cx))
    .await?;
    Ok(())
}

pub async fn logout(cx: &Cx) -> Result<()> {
    if let Some(hash) = session::stop(cx).await? {
        UserSession::filter_by_token_hash(hex(&hash)).delete().exec(&mut db(cx)).await?;
    }
    Ok(())
}

/// The session settings. With `SECURE_COOKIES=true` the cookie is topcoat's hardened
/// default (`__Host-`, `Secure`), which browsers drop over plain `http://` except on
/// localhost; the default is a plain `HttpOnly`, `SameSite=Lax` cookie for private
/// networks without HTTPS, like a homelab reached through a VPN.
///
/// A session ends `SESSION_DAYS` days after the user's last visit, a week by default.
pub fn session_config() -> session::SessionConfig {
    let secure = std::env::var("SECURE_COOKIES").is_ok_and(|v| matches!(v.trim(), "true" | "1"));
    let builder = session::SessionConfig::builder().lifetime(session_lifetime());
    if secure { builder.build() } else { builder.token_store(PlainCookieStore).build() }
}

/// How long a session lasts without a visit, when `SESSION_DAYS` is unset or invalid.
const DEFAULT_SESSION_DAYS: u64 = 7;

/// How long a session lasts without a visit, from `SESSION_DAYS`.
fn session_lifetime() -> Duration {
    static LIFETIME: LazyLock<Duration> = LazyLock::new(|| {
        let days = parse_session_days(std::env::var("SESSION_DAYS").ok().as_deref());
        Duration::from_secs(days * 24 * 60 * 60)
    });
    *LIFETIME
}

/// Browsers cap a cookie's age at 400 days, so a longer session would end there anyway.
fn parse_session_days(value: Option<&str>) -> u64 {
    value.and_then(|v| v.trim().parse().ok()).filter(|days| *days > 0).unwrap_or(DEFAULT_SESSION_DAYS).min(400)
}

/// The session cookie without `Secure`, so it survives plain `http://`.
struct PlainCookieStore;

/// Named after the app: browsers share cookies between every port of a host, so a
/// generic name like `session` is overwritten by any other app served from the same
/// machine that uses it, which signs the user out here.
const PLAIN_COOKIE: &str = "calibre_ui_session";

fn plain_cookies(cx: &Cx) -> impl topcoat::cookie::Cookies {
    use topcoat::cookie::{Cookies as _, SameSite, cookies};
    cookies(cx).override_same_site(SameSite::Lax).override_http_only(true).override_path("/")
}

impl session::TokenStore for PlainCookieStore {
    fn read<'a>(&'a self, cx: &'a Cx) -> session::TokenStoreFuture<'a, Option<session::Token>> {
        use topcoat::cookie::Cookies as _;
        Box::pin(async move {
            Ok(plain_cookies(cx).get(PLAIN_COOKIE).and_then(|c| session::Token::decode(c.value_trimmed()).ok()))
        })
    }

    fn write<'a>(
        &'a self,
        cx: &'a Cx,
        token: session::Token,
        max_age: std::time::Duration,
    ) -> session::TokenStoreFuture<'a, ()> {
        use topcoat::cookie::{Cookie, Cookies as _, time};
        Box::pin(async move {
            plain_cookies(cx)
                .override_max_age(time::Duration::try_from(max_age)?)
                .add(Cookie::new(PLAIN_COOKIE, token.encode()));
            Ok(())
        })
    }

    fn delete<'a>(&'a self, cx: &'a Cx) -> session::TokenStoreFuture<'a, ()> {
        use topcoat::cookie::{Cookie, Cookies as _};
        Box::pin(async move {
            plain_cookies(cx).remove(Cookie::new(PLAIN_COOKIE, ""));
            Ok(())
        })
    }
}

impl User {
    pub async fn list(cx: &Cx) -> Result<Vec<User>> {
        Ok(User::all().order_by(User::fields().name().asc()).exec(&mut db(cx)).await?)
    }

    pub async fn any_exist(cx: &Cx) -> Result<bool> {
        Ok(User::all().first().exec(&mut db(cx)).await?.is_some())
    }

    pub async fn get(cx: &Cx, id: u64) -> Result<Option<User>> {
        Ok(User::filter_by_id(id).first().exec(&mut db(cx)).await?)
    }

    /// Creates an account. Returns `false` if the name is taken.
    pub async fn add(cx: &Cx, name: &str, password: &str, perms: Perms) -> Result<bool> {
        let mut db = db(cx);
        if User::filter_by_name(name.to_owned()).first().exec(&mut db).await?.is_some() {
            return Ok(false);
        }
        toasty::create!(User {
            name,
            password_hash: hash_password(password)?,
            ereader_email: "",
            hidden_tags: "",
            kobo_token: "",
            admin: perms.admin,
            can_download: perms.download,
            can_upload: perms.upload,
            can_edit: perms.edit,
        })
        .exec(&mut db)
        .await?;
        Ok(true)
    }

    pub async fn set_perms(cx: &Cx, id: u64, perms: Perms) -> Result<()> {
        let mut db = db(cx);
        let mut user = User::get_by_id(&mut db, &id).await?;
        user.update()
            .admin(perms.admin)
            .can_download(perms.download)
            .can_upload(perms.upload)
            .can_edit(perms.edit)
            .exec(&mut db)
            .await?;
        Ok(())
    }

    /// Changes the password and signs the user out everywhere.
    pub async fn set_password(cx: &Cx, id: u64, password: &str) -> Result<()> {
        let mut db = db(cx);
        let mut user = User::get_by_id(&mut db, &id).await?;
        user.update().password_hash(hash_password(password)?).exec(&mut db).await?;
        UserSession::filter_by_user_id(id).delete().exec(&mut db).await?;
        Ok(())
    }

    /// The user whose Kobo sync URL carries this token.
    pub async fn by_kobo_token(cx: &Cx, token: &str) -> Result<Option<User>> {
        if token.is_empty() {
            return Ok(None);
        }
        Ok(User::filter_by_kobo_token(token.to_owned()).first().exec(&mut db(cx)).await?)
    }

    /// Gives the user a new Kobo sync token, which turns off the old URL, or turns Kobo
    /// sync off with `None`.
    pub async fn set_kobo_token(cx: &Cx, id: u64, token: Option<&str>) -> Result<()> {
        let mut db = db(cx);
        let mut user = User::get_by_id(&mut db, &id).await?;
        user.update().kobo_token(token.unwrap_or("")).exec(&mut db).await?;
        Ok(())
    }

    pub async fn set_hidden_tags(cx: &Cx, id: u64, tags: &str) -> Result<()> {
        let mut db = db(cx);
        let mut user = User::get_by_id(&mut db, &id).await?;
        user.update().hidden_tags(tags).exec(&mut db).await?;
        Ok(())
    }

    pub async fn set_ereader_email(cx: &Cx, id: u64, email: &str) -> Result<()> {
        let mut db = db(cx);
        let mut user = User::get_by_id(&mut db, &id).await?;
        user.update().ereader_email(email).exec(&mut db).await?;
        Ok(())
    }

    /// Deletes an account with its sessions.
    pub async fn remove(cx: &Cx, id: u64) -> Result<()> {
        let mut db = db(cx);
        UserSession::filter_by_user_id(id).delete().exec(&mut db).await?;
        User::filter_by_id(id).delete().exec(&mut db).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_basic_credentials() {
        assert_eq!(parse_basic("YWRtaW46c2VjcmV0"), Some(("admin".into(), "secret".into())));
        assert!(parse_basic("*").is_none());
        assert!(parse_basic("YQ==").is_none()); // no colon
    }

    #[test]
    fn reads_the_session_lifetime() {
        assert_eq!(parse_session_days(None), 7);
        assert_eq!(parse_session_days(Some(" 30 ")), 30);
        assert_eq!(parse_session_days(Some("")), 7);
        assert_eq!(parse_session_days(Some("0")), 7);
        assert_eq!(parse_session_days(Some("soon")), 7);
        assert_eq!(parse_session_days(Some("9999")), 400);
    }

    #[test]
    fn extends_sessions_once_they_have_aged() {
        let week = 7 * 24 * 60 * 60;
        // Just started or just extended: left alone.
        assert!(!due_for_extension(1000 + week, 1000, week));
        assert!(!due_for_extension(1000 + week - 600, 1000, week));
        assert!(due_for_extension(1000 + week - EXTEND_AFTER, 1000, week));
        // A session from before the lifetime was shortened waits until it fits.
        assert!(!due_for_extension(1000 + 4 * week, 1000, week));
    }

    #[test]
    fn hashes_and_verifies_passwords() {
        let hash = hash_password("hunter2").unwrap();
        assert!(verify_password("hunter2", &hash));
        assert!(!verify_password("hunter3", &hash));
    }

    #[test]
    fn admins_can_everything() {
        let mut user = User {
            id: 1,
            name: "a".into(),
            password_hash: String::new(),
            ereader_email: String::new(),
            hidden_tags: " Adult, ,spoilers ".into(),
            kobo_token: String::new(),
            admin: false,
            can_download: true,
            can_upload: false,
            can_edit: false,
        };
        assert!(user.can(Perm::Download) && !user.can(Perm::Edit) && !user.can(Perm::Admin));
        assert_eq!(user.hidden(), ["Adult", "spoilers"]);
        user.admin = true;
        assert!(user.can(Perm::Edit) && user.can(Perm::Admin));
    }
}
