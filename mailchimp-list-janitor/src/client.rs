use crate::{
    error::{ArchiveError, Error, FetchMemberError},
    models::{MailchimpError, MailchimpListResponse, MailchimpMember},
};
use async_gen::gen;
use futures_core::Stream;
use futures_util::StreamExt;
use reqwest::{IntoUrl, Url};
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

#[derive(Clone, Debug)]
pub struct Client {
    base_url: Url,
    list_id: String,
    api_key: String,
    http: reqwest::Client,
    max_concurrency: MaxConcurrency,
    page_size: PageSize,
}

/// A Mailchimp client that can fetch and archive unsubscribed users.
impl Client {
    /// Initializes a new client using default values for `max_concurrency` and `page_size`.
    ///
    /// ## Panic
    ///
    /// This function panics if the `base_url` is not a valid URL.
    pub fn new<U: IntoUrl, S: Into<String>>(base_url: U, list_id: S, api_key: S) -> Arc<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();
        Arc::new(Self {
            base_url: base_url.into_url().unwrap(),
            list_id: list_id.into(),
            api_key: api_key.into(),
            http,
            max_concurrency: Default::default(),
            page_size: Default::default(),
        })
    }

    /// Fetches all the unsubscribed users from the list using a stream.
    ///
    /// ## Example
    ///
    /// ```no_run
    /// use futures_util::StreamExt;
    /// use mailchimp_list_janitor::client::Client;
    /// use reqwest::Url;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::new("https://us2.api.mailchimp.com", "list-id", "api-abcd1234");
    ///     let stream = client.fetch_unsubscribed().await;
    ///
    ///     stream.for_each(|member| async move {
    ///         match member {
    ///             Ok(member) => {
    ///                 println!("Member: {:?}", member);
    ///             }
    ///             Err(err) => {
    ///                 eprintln!("Error: {:?}", err);
    ///             }
    ///         }
    ///     })
    ///     .await;
    /// }
    ///
    /// ```
    pub async fn fetch_unsubscribed(
        &self,
    ) -> impl Stream<Item = Result<MailchimpMember, FetchMemberError>> + '_ {
        let mut offset = 0;

        let g = gen! {
            loop {
                // Safe to unwrap here because the URL is mostly hardcoded (and the base URL is validated at construction time)
                let url = self.base_url.join(&format!(
                    "/3.0/lists/{}/members?status=unsubscribed&count={}&offset={}&sort_field=timestamp_signup&sort_dir=ASC",
                    self.list_id, self.page_size.0, offset
                )).unwrap();
                let resp = self.http
                  .get(url)
                  .basic_auth("anystring", Some(&self.api_key))
                  .send()
                  .await;

                let resp = match resp {
                    Ok(r) => r,
                    Err(e) => {
                        yield Err(FetchMemberError::Request(e));
                        break;
                    }
                };

                if resp.status().is_client_error() {
                    // Safe to unwrap because we know the response is an error (assuming Mailchimp doesn't change their error type)
                    let body: MailchimpError = resp.json().await.unwrap();
                    yield Err(FetchMemberError::Mailchimp(body));
                    break;
                }

                let body: MailchimpListResponse = resp.json().await.unwrap();

                if body.members.is_empty() {
                    break;
                }

                for member in body.members.into_iter() {
                    yield Ok(member);
                }

                offset += self.page_size.0;
            }

            ()
        };

        g.into_async_iter()
    }

    /// Archives an unsubscribed user from the list.
    ///
    /// ## Example
    ///
    /// ```no_run
    /// use mailchimp_list_janitor::client::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::new("https://us2.api.mailchimp.com", "list-id", "api-abcd1234");
    ///     let res = client.archive_unsubscribed("user-id").await;
    ///     match res {
    ///         Ok(id) => {
    ///             println!("Archived user with id {}", id);
    ///        }
    ///        Err(err) => {
    ///            eprintln!("Error: {:?}", err);
    ///        }
    ///    }
    /// }
    /// ```
    pub async fn archive_unsubscribed(&self, id: &str) -> Result<String, ArchiveError> {
        // Safe to unwrap here because the URL is mostly hardcoded (and the base URL is validated at construction time)
        let url = self
            .base_url
            .join(&format!("/3.0/lists/{}/members/{}", self.list_id, id))
            .unwrap();

        let resp = self
            .http
            .patch(url)
            .basic_auth("anystring", Some(&self.api_key))
            .header("Content-Type", "application/json")
            .body("{\"status\":\"cleaned\"}")
            .send()
            .await
            .map_err(|e| ArchiveError::Request(id.to_string(), e))?;

        if resp.status().is_client_error() {
            let body: MailchimpError = resp
                .json()
                .await
                .map_err(|e| ArchiveError::Request(id.to_string(), e))?;
            return Err(ArchiveError::Mailchimp(id.to_string(), body));
        }

        Ok(id.to_string())
    }

    async fn get_unsubscribed_ids(self: Arc<Self>) -> Result<Vec<String>, FetchMemberError> {
        let mut unsubcribed_ids = vec![];
        let mut unsubscribed_stream = self.fetch_unsubscribed().await.boxed();
        while let Some(res) = unsubscribed_stream.next().await {
            match res {
                Ok(member) => {
                    unsubcribed_ids.push(member.id);
                }
                Err(err) => {
                    return Err(err);
                }
            }
        }
        Ok(unsubcribed_ids)
    }

    /// Archives all the unsubscribed users from the list using a stream.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use futures_util::StreamExt;
    /// use mailchimp_list_janitor::client::Client;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Client::new("https://us2.api.mailchimp.com", "list-id", "api-abcd1234");
    ///     let stream = client.move_unsubscribed_to_archive().await?;
    ///
    ///     stream
    ///         .for_each(|res| async move {
    ///             match res {
    ///                 Ok(id) => println!("Archived user with id {}", id),
    ///                 Err(err) => eprintln!("{err}"),
    ///             }
    ///         })
    ///         .await;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn move_unsubscribed_to_archive(
        self: Arc<Self>,
    ) -> Result<impl Stream<Item = Result<String, Error>>, Error> {
        // Eagerly fetches all the ids of the unsubscribed users.
        // This is because mailchimp doesn't offer a stable paginated API, so if we
        // stream the user IDs as they arrive and we start deleting them, we might
        // end messing up the pagination and end up with missing records.
        let mut unsubcribed_ids = self.clone().get_unsubscribed_ids().await?;

        let g = gen! {
          let concurrency = self.max_concurrency.0.min(unsubcribed_ids.len());

          let mut tasks: JoinSet<Result<String, Error>> = JoinSet::new();
          while let Some(id) = unsubcribed_ids.pop() {
              let this = self.clone();
              tasks.spawn(async move {
                  this.archive_unsubscribed(&id).await.map_err(|e| e.into())
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
                      this.archive_unsubscribed(&id).await.map_err(|e| e.into())
                  });
              }
          }

          ()
        };

        Ok(g.into_async_iter())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header_exists, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn create_sample_member(id: u32) -> MailchimpMember {
        MailchimpMember {
            id: id.to_string(),
            email_address: format!("  test{}@test.com", id),
            unique_email_id: format!("unique-email-id-{}", id),
            contact_id: format!("contact-id-{}", id),
            full_name: format!("John Doe {}", id),
            web_id: 123,
            email_type: "html".into(),
            status: "unsubscribed".into(),
            unsubscribe_reason: "".into(),
            consents_to_one_to_one_messaging: false,
            merge_fields: Default::default(),
            interests: Default::default(),
            stats: Default::default(),
            ip_signup: format!("127.0.0.{}", id),
            timestamp_signup: "2021-01-01T00:00:00+00:00".into(),
            ip_opt: format!("127.0.0.{}", id),
            timestamp_opt: "2021-01-01T00:00:00+00:00".into(),
            member_rating: 0.0,
            last_changed: "2021-01-01T00:00:00+00:00".into(),
            language: "en".into(),
            vip: false,
            location: Default::default(),
            marketing_permissions: Default::default(),
            last_note: None,
            source: "API".into(),
            tags_count: 0,
            tags: Default::default(),
            list_id: "list-id".into(),
        }
    }

    #[tokio::test]
    async fn test_fetch_unsubscribed() {
        let mock_server = MockServer::start().await;
        let base_url = mock_server.uri();
        let client = Client {
            base_url: base_url.parse().unwrap(),
            list_id: "list-id".into(),
            api_key: "api-key".into(),
            http: reqwest::Client::new(),
            max_concurrency: MaxConcurrency::default(),
            page_size: PageSize(2),
        };

        Mock::given(method("GET"))
            .and(path("/3.0/lists/list-id/members"))
            .and(query_param("status", "unsubscribed"))
            .and(query_param("count", "2"))
            .and(query_param("offset", "0"))
            .and(query_param("sort_field", "timestamp_signup"))
            .and(query_param("sort_dir", "ASC"))
            .and(header_exists("Authorization"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(MailchimpListResponse {
                    members: vec![create_sample_member(1), create_sample_member(2)],
                }),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/3.0/lists/list-id/members"))
            .and(query_param("status", "unsubscribed"))
            .and(query_param("count", "2"))
            .and(query_param("offset", "2"))
            .and(query_param("sort_field", "timestamp_signup"))
            .and(query_param("sort_dir", "ASC"))
            .and(header_exists("Authorization"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(MailchimpListResponse {
                    members: vec![create_sample_member(3), create_sample_member(4)],
                }),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/3.0/lists/list-id/members"))
            .and(query_param("status", "unsubscribed"))
            .and(query_param("count", "2"))
            .and(query_param("offset", "4"))
            .and(query_param("sort_field", "timestamp_signup"))
            .and(query_param("sort_dir", "ASC"))
            .and(header_exists("Authorization"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(MailchimpListResponse { members: vec![] }),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let stream = client.fetch_unsubscribed().await;
        let ids = stream
            .map(|res| res.expect("request should be successful").id)
            .collect::<Vec<_>>()
            .await;

        assert_eq!(ids, vec!["1", "2", "3", "4"]);
    }
}
