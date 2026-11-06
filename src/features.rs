//! Application features. Each module holds everything one feature owns, such as its
//! data and business rules, for the pages in `app` to use.

pub mod auth;
pub mod calibre;
pub mod kobo;
pub mod ldap;
pub mod library;
pub mod mailer;
pub mod oidc;
pub mod shelf;
