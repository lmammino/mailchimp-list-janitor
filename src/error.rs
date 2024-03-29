use crate::models::MailchimpError;
use tokio::task::JoinError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    FetchIds(#[from] FetchIdsError),
    #[error("{0}")]
    Archive(#[from] ArchiveError),
    #[error("Task cancelled: {0}")]
    Join(#[from] JoinError),
}

#[derive(Debug, thiserror::Error)]
pub enum FetchIdsError {
    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),
    #[error("Mailchimp error: {0}")]
    Mailchimp(MailchimpError),
}

#[derive(Debug, thiserror::Error)]
pub enum ArchiveError {
    #[error("Request error while archiving user {0}: {1}")]
    Request(String, reqwest::Error),
    #[error("Mailchimp error while archiving user {0}: {1}")]
    Mailchimp(String, MailchimpError),
}
