# slk

A CLI tool to read Slack threads in the terminal.

## Usage

```bash
slk login                                # Authenticate via OAuth
slk list                                 # List conversations
slk history <channel-id>                 # Show recent messages in a channel
slk thread <channel-id> <thread-ts>      # Display thread messages
slk thread <url>                         # Display thread messages (from URL)
```

## Prerequisites

- Rust toolchain (for building)
- `curl` (used internally for Slack API calls)

## Installation

```bash
cargo install --path .
```

## Setup (Slack App)

1. Create a Slack app at https://api.slack.com/apps
2. Add OAuth redirect URL: `https://127.0.0.1:9876`
3. Add User Token Scopes: `channels:history`, `channels:read`, `groups:history`, `groups:read`, `mpim:read`, `im:read`, `users:read`
4. Note the Client ID and Client Secret

## Configuration

Set your Slack app credentials via environment variables:

```bash
export SLK_CLIENT_ID="..."
export SLK_CLIENT_SECRET="..."
```

Or create a config file at `~/.config/slk/config.json`:

```json
{ "client_id": "...", "client_secret": "..." }
```

Then run `slk login` to authenticate. The token is saved to `~/.config/slk/credentials`.

Alternatively, set the `SLACK_TOKEN` environment variable directly to skip the OAuth flow.
