# matrix-notify

Send one-off messages to your matrix account when compute jobs finish

## Building

`matrix-notify` can be installed through `cargo`:

```
git clone https://github.com/fluid-Dynamics-Group/matrix-notify
cd matrix-notify
```

You must create a `.config.json` file that specifies credentials to use matrix. It is *highly* recommended
that this is a shell user and *only* used for these one-off messages. 
**Do not use your usual matrix account credentials**.
The schema looks like this:

```json
{
    "matrix_username": "your-matrix-username",
    "matrix_password": "your-matrix-password",
    "homeserver_url": "https://matrix.org"
    "matrix_id": "@your-matrix-username:matrix.org"
}
```

## Usage

`matrix-notify` can send text messagse or attachments to your account when a job finishes.

```
matrix-notify help

Usage: matrix-notify <target_user> <command> [<args>]

Send matrix messages and attachments to specified users

Options:
  --help            display usage information

Commands:
  text              send a message with text content
  attachment        send a message with an attachment
```

Send a text message:

```
matrix-notify @your-username:homeserver-url text "message text here"
```

Send a file

```
matrix-notify @your-username:homeserver-url attachment /path/to/your/file
```
