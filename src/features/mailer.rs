//! "Send to e-reader": emails a book file to a user's e-reader address.
//!
//! Set `SMTP_URL` (such as `smtps://user:pass@smtp.example.com:465`) and `MAIL_FROM` to
//! deliver for real. Without `SMTP_URL`, messages are written as `.eml` files to
//! `mail-outbox/` for inspection.

use std::path::Path;

use topcoat::{
    Result,
    context::Cx,
    mail::{Attachment, FileTransport, MailConfig, SmtpTransport, mail, send},
};

/// The mail configuration from the environment.
pub fn config() -> MailConfig {
    match std::env::var("SMTP_URL") {
        Ok(url) => MailConfig::builder()
            .transport(SmtpTransport::from_url(&url).expect("SMTP_URL is not a valid SMTP URL").build())
            .build(),
        Err(_) => MailConfig::builder().transport(FileTransport::new("mail-outbox")).build(),
    }
}

fn sender() -> String {
    std::env::var("MAIL_FROM").unwrap_or_else(|_| "library@localhost".to_owned())
}

/// Emails `file` to `to` as an attachment named `name`.
pub async fn send_book(cx: &Cx, to: &str, title: &str, file: &Path, name: &str, content_type: &str) -> Result<()> {
    let bytes = tokio::fs::read(file).await?;
    let mail = mail! {
        from: sender(),
        to: to,
        subject: title,
        html: {
            cx =>
            <p>(crate::t!(mail_body, title))</p>
        },
        attachments: [Attachment::new(name, content_type, bytes)],
    }?;
    send(cx, mail).await?;
    Ok(())
}
