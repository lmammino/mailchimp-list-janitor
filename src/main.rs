use futures_util::StreamExt;
use mailchimp_list_janitor::client;
use std::env;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let api_key = env::var("MAILCHIMP_API_KEY")?;
    let base_url = env::var("MAILCHIMP_BASE_URL")?;
    let list_id = env::var("MAILCHIMP_LIST_ID")?;

    let client = client::Client::new(&base_url, &list_id, &api_key);
    let gen = client.move_unsubscribed_to_archive().await?;

    gen.for_each(|res| async move {
        match res {
            Ok(id) => println!("Archived user with id {}", id),
            Err(err) => eprintln!("{err}"),
        }
    })
    .await;

    Ok(())
}
