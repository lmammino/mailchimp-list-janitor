use clap::{Parser, Subcommand};
use futures_util::StreamExt;
use mailchimp_list_janitor::Client;

#[derive(Debug, Parser, Clone)]
#[command(name = "mailchimp-list-janitor")]
#[command(about = "A CLI tool to archive unsubscribed users from a mailchimp list")]
struct Cli {
    #[arg(short, long, env = "MAILCHIMP_API_KEY")]
    api_key: String,
    #[arg(short, long, env = "MAILCHIMP_BASE_URL")]
    base_url: String,
    #[arg(short, long, env = "MAILCHIMP_LIST_ID")]
    list_id: String,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand, Clone)]
enum Commands {
    #[command(about = "Archives all the unsubscribed users")]
    Archive,
    #[command(about = "Lists all the unsubscribed users")]
    List,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Cli::parse();

    let client = Client::new(&args.base_url, &args.list_id, &args.api_key);

    match args.command {
        Commands::Archive => {
            let gen = client.move_unsubscribed_to_archive().await?;

            gen.for_each(|res| async move {
                match res {
                    Ok(id) => println!("Archived user with id {}", id),
                    Err(err) => eprintln!("{err}"),
                }
            })
            .await;
        }
        Commands::List => {
            let gen = client.fetch_unsubscribed().await;
            println!("id,email_address,full_name");
            gen.for_each(|res| async move {
                match res {
                    Ok(member) => println!(
                        "{},{},\"{}\"",
                        member.id, member.email_address, member.full_name
                    ),
                    Err(err) => eprintln!("{err}"),
                }
            })
            .await;
        }
    }

    Ok(())
}
