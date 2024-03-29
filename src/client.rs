use crate::{
    error::{ArchiveError, Error, FetchIdsError},
    models::{MailchimpError, MailchimpListResponse},
};
use async_gen::gen;
use futures_core::Stream;
use std::{sync::Arc, time::Duration};
use tokio::task::JoinSet;

#[derive(Debug, Clone)]
pub struct PageSize(usize);

impl Default for PageSize {
    fn default() -> Self {
        Self(100)
    }
}

#[derive(Debug, Clone)]
pub struct MaxConcurrency(usize);

impl Default for MaxConcurrency {
    fn default() -> Self {
        Self(8)
    }
}

#[derive(Default, Clone, Debug)]
pub struct Client {
    base_url: String,
    list_id: String,
    api_key: String,
    http: reqwest::Client,
    max_concurrency: MaxConcurrency,
    page_size: PageSize,
}

impl Client {
    pub fn new<S: Into<String>>(base_url: S, list_id: S, api_key: S) -> Arc<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();
        Arc::new(Self {
            base_url: base_url.into(),
            list_id: list_id.into(),
            api_key: api_key.into(),
            http,
            ..Default::default()
        })
    }

    /// Eagerly fetches all the ids of the unsubscribed users.
    /// This is because mailchimp doesn't offer a stable paginated API, so if we
    /// stream the user IDs as they arrive and we start deleting them, we might
    /// end messing up the pagination and end up with missing records.
    async fn fetch_unsubscribed_ids(&self) -> Result<Vec<String>, FetchIdsError> {
        let mut offset = 0;
        let mut unsubcribed_ids = vec![];
        loop {
            let resp = self.http
              .get(&format!(
                  "{}/3.0/lists/{}/members?status=unsubscribed&count={}&offset={}&sort_field=timestamp_signup&sort_dir=ASC",
                  self.base_url, self.list_id, self.page_size.0, offset
              ))
              .basic_auth("anystring", Some(&self.api_key))
              .send()
              .await?;

            if resp.status().is_client_error() {
                let body: MailchimpError = resp.json().await?;
                return Err(FetchIdsError::Mailchimp(body));
            }

            let body: MailchimpListResponse = resp.json().await.unwrap();

            if body.members.is_empty() {
                break;
            }

            body.members.into_iter().for_each(|member| {
                unsubcribed_ids.push(member.id);
            });

            offset += self.page_size.0;
        }

        Ok(unsubcribed_ids)
    }

    async fn archive_unsubscribed(&self, id: String) -> Result<String, ArchiveError> {
        let resp = self
            .http
            .patch(&format!(
                "{}/3.0/lists/{}/members/{}",
                self.base_url, self.list_id, id
            ))
            .basic_auth("anystring", Some(&self.api_key))
            .header("Content-Type", "application/json")
            .body("{\"status\":\"cleaned\"}")
            .send()
            .await
            .map_err(|e| ArchiveError::Request(id.clone(), e))?;

        if resp.status().is_client_error() {
            let body: MailchimpError = resp
                .json()
                .await
                .map_err(|e| ArchiveError::Request(id.clone(), e))?;
            return Err(ArchiveError::Mailchimp(id, body));
        }

        Ok(id)
    }

    pub async fn move_unsubscribed_to_archive(
        self: Arc<Self>,
    ) -> Result<impl Stream<Item = Result<String, Error>>, Error> {
        let mut unsubcribed_ids = self.fetch_unsubscribed_ids().await?;

        let g = gen! {
          let concurrency = self.max_concurrency.0.min(unsubcribed_ids.len());

          let mut tasks: JoinSet<Result<String, Error>> = JoinSet::new();
          while let Some(id) = unsubcribed_ids.pop() {
              let this = self.clone();
              tasks.spawn(async move {
                  this.archive_unsubscribed(id).await.map_err(|e| e.into())
              });

              if tasks.len() >= concurrency {
                  break;
              }
          }

          while let Some(res) = tasks.join_next().await {
              match res {
                Ok(r) => {
                    yield r;
                },
                Err(err) => {
                    yield Err(Error::Join(err));
                }
              }

              let this = self.clone();
              if let Some(id) = unsubcribed_ids.pop() {
                  tasks.spawn(async move {
                      this.archive_unsubscribed(id).await.map_err(|e| e.into())
                  });
              }
          }

          ()
        };

        Ok(g.into_async_iter())
    }
}
