# DiscFS

Store files as attachments on Discord messages.

# Installation

Install with `cargo install discfs` or using the Nix flake in the project repository.

Note that you will need development libraries `fuse3-dev`, `openssl-dev` and `pkg-config` or whatever they are called in your package manager.

# Setup

## Creating the Discord bot
First you need to create a Discord bot to upload the files.
Following the documentation (https://discord.com/developers/docs/getting-started), create a bot.
Under the sidebar "OAuth2 -> URL Generator", give it the `bot` scope and following bot permissions:

- Send Messages
- Attach Files
- Read Message History

Then copy the generated url at the bottom, paste it in the browser and install the bot in your personal server.

Next, get the bot token by going to sidebar "Bot", clicking "Reset Token" and copying the token.

This token as well as the id of the channel you want the bot to send the messages in needs to be set in the following environment variables:

```.env
DISCORD_TOKEN=
CHANNEL_ID=
```

Channel ID can be retrieved by going to the Discord web application, navigating to the channel, and copying the last part of the url.

```
https://discord.com/channels/956113749209661480/ -> 956113749209661483 <- (this part)
```

## Running the CLI

Usage text is as follows:

```
Usage: discfs [OPTIONS] <MOUNTPOINT>

Arguments:
  <MOUNTPOINT>  Path to mount virtual filesystem at

Options:
      --dotenv             Use dotenv-vault (https://www.dotenv.org/docs/)
  -v...                    Logging verbosity. Repeat multiple times to increase logging level
      --db-path <DB_PATH>  Path to create SQLite database file [env: DB_PATH=fs.db] [default: ./fs.db]
  -h, --help               Print help
  -V, --version            Print version
```

Make sure you don't accidently delete the SQLite database as that maps all the attachments and stores all the file metadata.
Deleting it will lead to all uploaded content being unreachable.