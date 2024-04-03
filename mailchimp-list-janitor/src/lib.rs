//! A client for fetching and archiving unsubscribed users from a Mailchimp list.
//!
//! ## Example
//!
//! ```no_run
//! use futures_util::StreamExt;
//! use mailchimp_list_janitor::Client;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>>{
//!     let client = Client::new("https://us2.api.mailchimp.com", "list-id", "api-abcd1234");
//!     let stream = client.move_unsubscribed_to_archive().await?;
//!
//!      stream
//!          .for_each(|res| async move {
//!              match res {
//!                  Ok(id) => println!("Archived user with id {}", id),
//!                  Err(err) => eprintln!("{err}"),
//!              }
//!          })
//!          .await;
//!
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod error;
pub mod models;

pub use client::Client;
