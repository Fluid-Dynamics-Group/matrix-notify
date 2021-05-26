# matrix-notify

Send one-off messages to your matrix account when compute jobs finish

## Building

`matrix-notify` can be installed through `cargo`:

```
cargo install matrix-notify
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
