use ruma_client::Client;
use ruma_client_api::r0::media::create_content;
use ruma_client_api::r0::membership::{get_member_events, joined_rooms};
use ruma_client_api::r0::message::create_message_event;
use ruma_client_api::r0::room::create_room;
use ruma_events::room::member::MembershipState;
use ruma_identifiers::UserId;
use std::convert::TryFrom;
use std::io::Read;

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
    let self_user_id = UserId::try_from("@compute-notify:matrix.org")?;

    let target_user = UserId::try_from(args.target_user.clone())
        .map_err(|_| Error::UsernameErr(args.target_user.clone()))?;

    let config = ConfigInfo::new()?;

    let client = Client::https(url::Url::parse(&config.homeserver_url)?, None);

    client
        .log_in(config.matrix_username, config.matrix_password, None, None)
        .await?;

    //leave_all_rooms(&client).await.unwrap();

    let mut user_room = None;

    let rooms = joined_rooms::Request {};
    let rooms_response = client.request(rooms).await?;

    for joined_room in rooms_response.joined_rooms.into_iter() {
        let membership_request = get_member_events::Request {
            room_id: joined_room.clone(),
            at: None,
            membership: None,
            not_membership: None,
        };
        let membership_response = client.request(membership_request).await?;

        let mut target_not_leave = true;

        for chunk in membership_response.chunk {
            let chunk = chunk.deserialize()?;
            if chunk.sender == self_user_id {
                continue;
            } else if chunk.sender == target_user {
                match chunk.content.membership {
                    MembershipState::Ban => {
                        target_not_leave = false;
                        break;
                    }
                    MembershipState::Leave => {
                        target_not_leave = false;
                        break;
                    }
                    _ => (),
                }
            } else {
                break;
            }
        }

        if target_not_leave {
            user_room = Some(joined_room);
            break;
        }
    }

    // fetch the room ID that the user is currently in
    let room_id = if let Some(room_id) = user_room {
        // send message to this room
        room_id
    } else {
        //we must now create a room and send messages to it
        let create_room_request = create_room::Request {
            creation_content: None,
            initial_state: vec![],
            invite: vec![target_user],
            invite_3pid: vec![],
            is_direct: Some(true),
            name: Some("compute-notify".to_string()),
            power_level_content_override: None,
            preset: Some(create_room::RoomPreset::PrivateChat),
            //room_alias_name: Some("compute-notify".to_string()),
            room_alias_name: None,
            room_version: None,
            topic: None,
            visibility: None,
        };

        let response = client.request(create_room_request).await?;

        response.room_id
    };

    let txn_id = String::new();

    match args.subcommands {
        Subcommands::Text(text) => {
            let data = TextMessage::new(text.text);
            let text_request = create_message_event::Request {
                room_id,
                event_type: ruma_events::EventType::RoomMessage,
                txn_id,
                data: data.to_raw(),
            };

            client.request(text_request).await?;
        }
        Subcommands::Attachment(attachment) => {
            let mime = mime_guess::from_path(&attachment.path)
                .first_or_octet_stream()
                .essence_str()
                .to_string();
            let pathbuf = std::path::PathBuf::from(attachment.path);
            let filename = pathbuf
                .file_name()
                .ok_or(Error::MissingFilename)?
                .to_str()
                .unwrap_or("file")
                .to_string();

            let mut reader = std::fs::File::open(&pathbuf)?;
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes)?;
            let size = bytes.len();

            let upload_request = create_content::Request {
                filename: Some(filename.clone()),
                content_type: mime.clone(),
                file: bytes,
            };

            let uri = client.request(upload_request).await?.content_uri;

            let attachment = AttachmentMessage::new(uri, size, mime, filename)?;

            let file_request = create_message_event::Request {
                room_id,
                event_type: ruma_events::EventType::RoomMessage,
                txn_id,
                data: attachment.to_raw(),
            };

            client.request(file_request).await?;
        }
    }

    Ok(())
}

#[derive(serde::Serialize)]
struct TextMessage {
    body: String,
    format: &'static str,
    formatted_body: String,
    msgtype: &'static str,
}
impl TextMessage {
    fn new(body: String) -> Self {
        Self {
            body: body.clone(),
            format: "org.matrix.custom.html",
            formatted_body: body,
            msgtype: "m.text",
        }
    }

    fn to_raw(self) -> Box<serde_json::value::RawValue> {
        let string = serde_json::to_string(&self).unwrap();
        serde_json::value::RawValue::from_string(string).unwrap()
    }
}

#[derive(serde::Serialize)]
struct AttachmentMessage {
    body: String,
    ///The original filename of the uploaded file.
    filename: String,
    ///Information about the file referred to in url.
    info: FileInfo,
    msgtype: &'static str,
    url: String,
}
impl AttachmentMessage {
    fn new(file_url: String, size: usize, mime: String, filename: String) -> Result<Self, Error> {
        let info = FileInfo {
            mimetype: mime,
            size: size,
            thumbnail_url: None,
            thumbnail_info: None,
        };

        Ok(Self {
            body: filename.clone(),
            ///The original filename of the uploaded file.
            filename: filename,
            ///Information about the file referred to in url.
            info,
            msgtype: "",
            url: file_url,
        })
    }

    fn to_raw(self) -> Box<serde_json::value::RawValue> {
        let string = serde_json::to_string(&self).unwrap();
        serde_json::value::RawValue::from_string(string).unwrap()
    }
}

#[derive(serde::Serialize)]
struct FileInfo {
    mimetype: String,
    size: usize,
    thumbnail_url: Option<String>,
    thumbnail_info: Option<String>,
}

//#[allow(dead_code)]
//async fn leave_all_rooms(TODO) -> Result<(), Error> {
// let req = ruma_client_api::r0::membership::leave_room::Request {
//     room_id: joined_room,
// };

// client.request(req).await?;
//    Ok(())
//}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("A generic matrix api error occured. Full error: `{0}`")]
    MatrixError(#[from] ruma_client::Error<ruma_client_api::Error>),
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
    #[error("adf")]
    DeserializeError(#[from] ruma_events::InvalidEvent),
    #[error("adf")]
    IdentifiersError(#[from] ruma_identifiers::Error),
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
