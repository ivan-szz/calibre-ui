pub mod users;

use serde::Deserialize;

use crate::features::auth::Perms;

/// Permission checkboxes as a form sends them: present when checked.
#[derive(Deserialize)]
pub struct PermsForm {
    admin: Option<String>,
    download: Option<String>,
    upload: Option<String>,
    edit: Option<String>,
}

impl PermsForm {
    pub fn perms(&self) -> Perms {
        Perms {
            admin: self.admin.is_some(),
            download: self.download.is_some(),
            upload: self.upload.is_some(),
            edit: self.edit.is_some(),
        }
    }
}
