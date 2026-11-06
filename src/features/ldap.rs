//! Signing in against an LDAP directory, as calibre-web offers.
//!
//! Set `LDAP_URL` (such as `ldaps://ldap.example.com` or `ldap://localhost:389`) and
//! `LDAP_USER_DN`, the user's DN with `{username}` where the name goes, such as
//! `uid={username},ou=people,dc=example,dc=com`. A sign-in binds as that DN with the
//! typed password; LDAP accounts get a local account on first sign-in.

use ldap3::LdapConnAsync;
use topcoat::{Error, Result};

/// The URL and DN template, when LDAP is configured.
fn config() -> Option<(String, String)> {
    let url = std::env::var("LDAP_URL").ok().filter(|u| !u.is_empty())?;
    let dn = std::env::var("LDAP_USER_DN").ok().filter(|d| d.contains("{username}"))?;
    Some((url, dn))
}

/// Escapes a value for use in a DN, as RFC 4514 requires, so a user name cannot
/// change which entry is bound.
fn escape_dn(value: &str) -> String {
    let mut out = String::new();
    let last = value.chars().count().saturating_sub(1);
    for (i, c) in value.chars().enumerate() {
        match c {
            ',' | '+' | '"' | '\\' | '<' | '>' | ';' | '=' => {
                out.push('\\');
                out.push(c);
            }
            '#' if i == 0 => out.push_str("\\#"),
            ' ' if i == 0 || i == last => out.push_str("\\ "),
            '\0' => out.push_str("\\00"),
            _ => out.push(c),
        }
    }
    out
}

/// Whether the directory accepts this name and password. `false` when LDAP is not
/// configured.
pub async fn bind(name: &str, password: &str) -> Result<bool> {
    // An empty password makes an unauthenticated bind, which servers accept for any
    // DN, so it must never count as a sign-in.
    if password.is_empty() || name.is_empty() {
        return Ok(false);
    }
    let Some((url, template)) = config() else {
        return Ok(false);
    };
    let dn = template.replace("{username}", &escape_dn(name));
    let (conn, mut ldap) = LdapConnAsync::new(&url)
        .await
        .map_err(|e| Error::msg(format!("could not reach the LDAP server: {e}")))?;
    ldap3::drive!(conn);
    let accepted = ldap
        .simple_bind(&dn, password)
        .await
        .map_err(|e| Error::msg(format!("LDAP bind failed: {e}")))?
        .success()
        .is_ok();
    let _ = ldap.unbind().await;
    Ok(accepted)
}

#[cfg(test)]
mod tests {
    use super::{bind, escape_dn};

    #[tokio::test]
    async fn never_accepts_an_empty_password() {
        // Returns before reading the configuration or touching the network.
        assert!(!bind("ada", "").await.unwrap());
        assert!(!bind("", "secret").await.unwrap());
    }

    #[test]
    fn escapes_dn_values() {
        assert_eq!(escape_dn("ada"), "ada");
        assert_eq!(escape_dn("a,ou=admins"), "a\\,ou\\=admins");
        assert_eq!(escape_dn("#x "), "\\#x\\ ");
        assert_eq!(escape_dn(" x"), "\\ x");
    }
}
