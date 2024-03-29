use serde::Deserialize;
use std::fmt::Display;

#[derive(Deserialize, Clone, Debug)]
pub struct MailchimpMember {
    pub id: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct MailchimpListResponse {
    pub members: Vec<MailchimpMember>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MailchimpError {
    pub r#type: String,
    pub title: String,
    pub status: u16,
    pub detail: String,
    pub istance: String,
}

impl Display for MailchimpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("{} ({}): {}", self.title, self.status, self.detail).as_str())
    }
}
