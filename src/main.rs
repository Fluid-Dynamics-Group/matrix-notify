use matrix_sdk::api::r0::room::create_room;
use matrix_sdk::events::room::message::MessageEventContent;
use matrix_sdk::events::AnyMessageEventContent;
use matrix_sdk::identifiers::UserId;
use matrix_sdk::Client;
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

#[tokio::main]
async fn main() {
    if let Err(e) = inner_main().await {
        eprintln!("Error: {}", e)
    }
}

async fn inner_main() -> Result<(), Error> {
    let args: Args = argh::from_env();

    let target_user = UserId::try_from(args.target_user.clone())
        .map_err(|_| Error::UsernameErr(args.target_user.clone()))?;

    let config = ConfigInfo::new()?;

    let client = Client::new(url::Url::parse(&config.homeserver_url)?)?;

    let device_id = None;
    let _self_user_id = client
        .login(
            &config.matrix_username,
            &config.matrix_password,
            device_id,
            Some("matrix-notify"),
        )
        .await?
        .user_id;

    //client.sync_once(matrix_sdk::SyncSettings::new()).await?;
    //leave_all_rooms(&client).await.unwrap();
    client.sync_once(matrix_sdk::SyncSettings::new()).await?;

    let mut user_room = None;

    for room in client.joined_rooms() {
        match room.get_member(&target_user).await? {
            Some(_) => {
                user_room = Some(room);
                break;
            }
            _ => continue,
        }
        //
    }

    // fetch the room the user is in or create a new one
    let room = if let Some(room) = user_room {
        // send message to this room
        room
    } else {
        //we must now create a room and send messages to it
        let mut request = create_room::Request::new();
        request.is_direct = true;
        let invites = [target_user];
        request.invite = &invites;
        request.name = Some("compute-notify");

        let room_id = client.create_room(request).await?.room_id;

        client
            .sync_with_callback(matrix_sdk::SyncSettings::new(), |response| async move {
                if response.rooms.join.len() > 0 {
                    matrix_sdk::LoopCtrl::Break
                } else {
                    matrix_sdk::LoopCtrl::Continue
                }
            })
            .await;

        let room = client.get_room(&room_id).expect("room that we just created is not in the client's rooms. This should not happen. Report this issue, it is a bug");

        if let matrix_sdk::room::Room::Joined(joined_room) = room {
            joined_room
        } else {
            panic!("We have left the room that we created. This should not happen, it is a bug. Please report this issue");
        }
    };

    match args.subcommands {
        Subcommands::Text(text) => {
            let text =
                AnyMessageEventContent::RoomMessage(MessageEventContent::text_plain(text.text));

            room.send(text, None).await?;
        }
        Subcommands::Attachment(attachment) => {
            let pathbuf = std::path::PathBuf::from(&attachment.path);

            let mime = mime_guess::from_path(&attachment.path).first_or_octet_stream();
            let file_name = pathbuf.file_name().ok_or(Error::MissingFilename)?;
            let mut reader = std::fs::File::open(&pathbuf)?;

            room.send_attachment(
                file_name.to_str().unwrap_or(&attachment.path),
                &mime,
                &mut reader,
                None,
            )
            .await?;
        }
    }

    Ok(())
}

#[allow(dead_code)]
async fn leave_all_rooms(client: &matrix_sdk::Client) -> Result<(), Error> {
    for room in client.joined_rooms() {
        println!("leaving room");
        room.leave().await?;
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("A generic matrix api error occured. Full error: `{0}`")]
    MatrixSdkError(#[from] matrix_sdk::Error),
    #[error("Failed to parse a url from the provided homeserver url in the .config.json")]
    UrlError(#[from] url::ParseError), //
    #[error("Failed to parse a matrix user id from `{0}`. Example user id: @username:matrix-org")]
    UsernameErr(String),
    #[error("The provided attachment path was a directory, not a file")]
    MissingFilename,
    #[error("Error opening the provided attachment path: `{0}`")]
    IoError(#[from] std::io::Error),
    #[error("Could not deserialize config file: `{0}`")]
    SerdeError(#[from] serde_json::error::Error),
}

#[derive(serde::Deserialize)]
struct ConfigInfo {
    matrix_username: String,
    matrix_password: String,
    homeserver_url: String,
}
impl ConfigInfo {
    fn new() -> Result<Self, Error> {
        let bytes = include_bytes!("../.config.json");
        let text = String::from_utf8(bytes.to_vec()).expect("input json was not utf8");
        let x = serde_json::from_str(&text)?;
        Ok(x)
    }
}
