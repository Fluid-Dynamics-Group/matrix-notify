use matrix_notify::Error;

use ruma::identifiers::UserId;
use std::convert::TryFrom;

#[derive(argh::FromArgs)]
/// Send matrix messages and attachments to specified users
struct Args {
    #[argh(positional)]
    /// matrix username (including homeserver url) to send the message to.
    ///
    /// Example: @username:matrix.org
    target_user: String,

    /// the text content of the message to send
    #[argh(subcommand)]
    subcommands: Subcommands,
}

#[derive(argh::FromArgs)]
#[argh(subcommand)]
enum Subcommands {
    Text(Text),
    Attachment(Attachment),
}

#[derive(argh::FromArgs)]
#[argh(subcommand, name = "text")]
/// send a message with text content
struct Text {
    #[argh(positional)]
    /// content of the message to send
    text: String,
}

#[derive(argh::FromArgs)]
#[argh(subcommand, name = "attachment")]
/// send a message with an attachment
struct Attachment {
    #[argh(positional)]
    /// content of the message to send
    path: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(e) = inner_main().await {
        eprintln!("Error: {}", e)
    }
}

async fn inner_main() -> Result<(), Error> {
    let args: Args = argh::from_env();
    let self_user_id = UserId::try_from("@compute-notify:matrix.org")?;

    let target_user = UserId::try_from(args.target_user.clone())
        .map_err(|_| Error::UsernameErr(args.target_user.clone()))?;

    let config = matrix_notify::ConfigInfo::new()?;

    let client = matrix_notify::client(&config).await?;

    match args.subcommands {
        Subcommands::Text(text) => {
            matrix_notify::send_text_message(&client, text.text, target_user, self_user_id).await?;
        }
        Subcommands::Attachment(attachment) => {
            matrix_notify::send_attachment(
                &client,
                &attachment.path,
                None,
                target_user,
                self_user_id,
            )
            .await?;
        }
    }

    Ok(())
}
