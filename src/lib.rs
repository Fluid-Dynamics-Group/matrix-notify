use std::convert::TryFrom;
use std::io::Read;
#[cfg(feature = "cli")]
use {
    ruma::api::client::r0::media::create_content,
    ruma::api::client::r0::membership::{get_member_events, joined_rooms},
    ruma::api::client::r0::message::send_message_event,
    ruma::api::client::r0::room::create_room,
    ruma::events as ruma_events,
    ruma::events::room::member::MembershipState,
    ruma::identifiers::RoomId,
};

#[cfg(feature = "userid")]
pub use ruma_identifiers::UserId;

#[cfg(feature = "cli")]
pub type HyperClient = ruma::client::http_client::HyperNativeTls;
#[cfg(feature = "cli")]
pub type Client = ruma::client::Client<HyperClient>;
#[cfg(feature = "cli")]
type RumaClientError = ruma::client::Error<hyper::Error, ruma::api::client::Error>;

#[cfg(feature = "cli")]
pub async fn send_text_message(
    client: &Client,
    text: String,
    target_user: UserId,
    self_id: UserId,
) -> Result<(), Error> {
    let room_id = get_room_id(client, target_user, self_id).await?;

    let txn_id = String::new();

    let data = text_event(text);

    let text_request = send_message_event::Request::new(&room_id, &txn_id, &data);

    client.send_request(text_request).await?;

    Ok(())
}

#[cfg(feature = "cli")]
pub async fn client(config: &ConfigInfo) -> Result<Client, Error> {
    let https = hyper_tls::HttpsConnector::new();
    let client = hyper::Client::builder().build::<_, hyper::Body>(https);

    let client = Client::with_http_client(client, config.homeserver_url.clone(), None);

    client
        .log_in(&config.matrix_username, &config.matrix_password, None, None)
        .await?;

    Ok(client)
}

#[cfg(feature = "cli")]
fn text_event(text: String) -> ruma_events::AnyMessageEventContent {
    let text = ruma_events::room::message::TextMessageEventContent::plain(text);
    let msg_type = ruma_events::room::message::MessageType::Text(text);
    let message_event = ruma_events::room::message::MessageEventContent::new(msg_type);
    ruma_events::AnyMessageEventContent::RoomMessage(message_event)
}

#[cfg(feature = "cli")]
pub async fn send_attachment(
    client: &Client,
    attachment_path: &str,
    description: Option<String>,
    target_user: UserId,
    self_id: UserId,
) -> Result<(), Error> {
    let room_id = get_room_id(client, target_user, self_id).await?;

    let mime = mime_guess::from_path(attachment_path)
        .first_or_octet_stream()
        .essence_str()
        .to_string();

    let pathbuf = std::path::PathBuf::from(attachment_path);

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

    let upload_request = create_content::Request::new(&bytes);

    let uri = client.send_request(upload_request).await?.content_uri;

    let description = description.unwrap_or("".to_string());

    // TODO: load info into the struct here using the above data

    let msg_type = if mime.starts_with("video") {
        println!("sending as video");
        send_as_video(description, uri, filename, mime, size)
    } else if mime.starts_with("image") {
        println!("sending as image");
        send_as_image(description, uri, filename, mime, size)
    } else {
        println!("sending as file");
        send_as_file(description, uri, filename, mime, size)
    };

    let message_event = ruma_events::room::message::MessageEventContent::new(msg_type);
    let any_file_event = ruma_events::AnyMessageEventContent::RoomMessage(message_event);

    let txn_id = String::new();

    let file_request = send_message_event::Request::new(&room_id, &txn_id, &any_file_event);

    client.send_request(file_request).await?;

    Ok(())
}

#[cfg(feature = "cli")]
fn send_as_file(
    description: String,
    uri: ruma::MxcUri,
    filename: String,
    mime: String,
    size: usize,
) -> ruma::events::room::message::MessageType {
    let mut info = Box::new(ruma::events::room::message::FileInfo::new());
    info.mimetype = Some(mime);
    info.size = Some((size as u32).into());

    let mut file =
        ruma_events::room::message::FileMessageEventContent::plain(description, uri, Some(info));
    file.filename = Some(filename.clone());
    file.body = filename;

    let msg_type = ruma_events::room::message::MessageType::File(file);

    msg_type
}

// TODO: make this not render like that
#[cfg(feature = "cli")]
fn send_as_video(
    description: String,
    uri: ruma::MxcUri,
    filename: String,
    mime: String,
    size: usize,
) -> ruma::events::room::message::MessageType {
    let height = 868_u32.into();
    let width = 800_u32.into();
    let size = (size as u32).into();

    let mut info = Box::new(ruma::events::room::message::VideoInfo::new());
    info.mimetype = Some(mime);
    info.size = Some(size);
    info.height = Some(height);
    info.width = Some(width);

    let mut file =
        ruma_events::room::message::VideoMessageEventContent::plain(description, uri, Some(info));
    file.body = filename;

    let msg_type = ruma_events::room::message::MessageType::Video(file);

    msg_type
}

#[cfg(feature = "cli")]
fn send_as_image(
    description: String,
    uri: ruma::MxcUri,
    filename: String,
    mime: String,
    size: usize,
) -> ruma::events::room::message::MessageType {
    let height = 868_u32.into();
    let width = 800_u32.into();
    let size = (size as u32).into();

    let mut info = Box::new(ruma::events::room::ImageInfo::new());
    info.mimetype = Some(mime);
    info.size = Some(size);

    // TODO: pull some actual width and height information
    info.height = Some(height);
    info.width = Some(width);
    info.thumbnail_url = Some(uri.clone());

    let mut file =
        ruma_events::room::message::ImageMessageEventContent::plain(description, uri, Some(info));
    file.body = filename;

    let msg_type = ruma_events::room::message::MessageType::Image(file);

    msg_type
}

#[cfg(feature = "cli")]
async fn get_room_id(
    client: &Client,
    target_user: UserId,
    self_id: UserId,
) -> Result<RoomId, Error> {
    let room_id = if let Some(id) = find_room(client, target_user.clone(), self_id).await? {
        id
    } else {
        create_room(client, target_user).await?
    };

    Ok(room_id)
}

#[cfg(feature = "cli")]
async fn find_room(
    client: &Client,
    target_user: UserId,
    self_user_id: UserId,
) -> Result<Option<RoomId>, Error> {
    let mut user_room = None;

    let rooms = joined_rooms::Request::new();
    let rooms_response: joined_rooms::Response = client.send_request(rooms).await?;

    for joined_room in rooms_response.joined_rooms.into_iter() {
        let membership_request = get_member_events::Request::new(&joined_room);

        let membership_response = client.send_request(membership_request).await?;

        let mut target_not_leave = false;

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
                    _ => target_not_leave = true,
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

    Ok(user_room)
}

#[cfg(feature = "cli")]
fn make_name<'a>(x: &'a str) -> Result<&'a ruma::identifiers::RoomName, ruma::identifiers::Error> {
    TryFrom::try_from(x)
}

#[cfg(feature = "cli")]
async fn create_room(client: &Client, target_user: UserId) -> Result<RoomId, Error> {
    let name = "compute-notify";
    let room_name = make_name(name).unwrap();

    let creation_content = ruma::api::client::r0::room::create_room::CreationContent::new();

    //we must now create a room and send messages to it
    let mut create_room_request = create_room::Request::new();
    let target = &[target_user];

    create_room_request.creation_content = creation_content;
    create_room_request.initial_state = &[];
    create_room_request.invite = target;
    create_room_request.invite_3pid = &[];
    create_room_request.is_direct = true;
    create_room_request.name = Some(&room_name);
    create_room_request.power_level_content_override = None;
    create_room_request.preset = Some(create_room::RoomPreset::PrivateChat);
    create_room_request.room_alias_name = None;
    create_room_request.room_version = None;
    create_room_request.topic = None;
    create_room_request.visibility = ruma::api::client::r0::room::Visibility::Private;

    let response = client.send_request(create_room_request).await?;

    Ok(response.room_id)
}

#[cfg(feature = "cli")]
#[derive(Debug, thiserror::Error)]
pub enum Error {
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
    #[error("Error with the client: `{0}`")]
    RumaClient(#[from] RumaClientError),
    #[error("Could not resolve identifier: `{0}`")]
    RumaIdentifier(#[from] ruma::identifiers::Error),
    #[error("There was a hyper error: `{0}`")]
    HyperError(
        #[from] ruma::client::Error<hyper::Error, ruma::api::client::r0::uiaa::UiaaResponse>,
    ),
}

#[cfg(feature = "cli")]
#[derive(serde::Deserialize)]
pub struct ConfigInfo {
    pub matrix_username: String,
    pub matrix_password: String,
    pub homeserver_url: String,
}

#[cfg(feature = "cli")]
impl ConfigInfo {
    pub fn new() -> Result<Self, Error> {
        let bytes = include_bytes!("../.config.json");
        let text = String::from_utf8(bytes.to_vec()).expect("input json was not utf8");
        let x = serde_json::from_str(&text)?;
        Ok(x)
    }
}
