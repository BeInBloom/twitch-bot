# Twitch Bot

Twitch bot workspace in Rust with a small staged event pipeline:

- Twitch EventSub is the input source
- incoming SDK events are mapped into domain `Event`
- `Event` is parsed into typed requests (`ChatRequest`, `CommandRequest`, `RewardRequest`) and dispatched statically with exhaustive `match`
- routers dispatch by stage
- handlers execute bot behavior

The current executable lives in `crates/twitch-bot`. The workspace also contains a small Twitch SDK and config helper macros.

## Current behavior

What the bot actually does today:

- consumes Twitch EventSub events through `twitch-sdk`
- supports two domain event kinds in the main pipeline:
  - chat messages
  - channel point reward redemptions
- converts unsupported SDK events and chat messages without a complete target into `Unsupported` events
- routes chat messages into:
  - plain messages
  - commands
- supports two registered commands:
  - `!music`
  - `!skip`
- logs and ignores unknown commands
- logs redemptions of rewards unknown to the bot
- logs plain chat messages at `trace`
- logs unsupported events at `warn`

Command behavior:

- `!music` calls `playerctl metadata` and sends `сейчас играет трек <artist> - <title>` to chat, or `сейчас ничего не играет` when no player is running
- `!skip` calls `playerctl next` and sends `переключил трек` to chat

## Workspace layout

```text
crates/
├── macros/         # proc-macro helpers used by config wrapper types
├── macros-core/    # shared error types for macros
├── twitch-bot/     # application crate
└── twitch-sdk/     # Twitch EventSub/chat/auth SDK
```

Important application modules:

```text
crates/twitch-bot/src/
├── adapters/       # Twitch + system integrations
├── app/
│   ├── command.rs  # chat command parsing
│   ├── dispatch/   # handler trait, routers, typed requests
│   ├── handlers/   # leaf handlers
│   └── ports/      # application-facing interfaces
├── config/         # YAML config loading and wrapper-based validation
├── model/          # domain events and supporting types
├── runtime/        # consumer, logging, shutdown, supervisor
├── bootstrap.rs    # composition root
└── main.rs         # process entrypoint
```

## Architecture

The current codebase is not organized as `core/domain/infra` directories anymore. It is organized around the runtime pipeline and application stages.

High-level flow:

```text
main
  -> LogGuard::init
  -> bootstrap::run
  -> ConfigFile::load
  -> TokenManager background refresh loop (rotated refresh token is written back to config.yaml)
  -> adapters + routers + handlers wiring
  -> Supervisor::run
  -> EventSource::fetch
  -> Consumer<Event>::consume
  -> EventRouter<Event>
       -> ChatRequest -> ChatRouter<ChatRequest>
            -> CommandRouter<CommandRequest>
       -> RewardRequest -> RewardRouter<RewardRequest>
       -> UnsupportedEvent -> UnsupportedEventHandler
```

The staged dispatch tree is documented visually in:

- `docs/architecture/event-pipeline.png`
- `docs/architecture/event-pipeline.svg`
- `docs/architecture/event-pipeline.mmd`

### Runtime details

The runtime behavior currently implemented in code:

- `Consumer` processes up to `30` events concurrently
- each event handler execution has a `10s` timeout
- graceful shutdown waits up to `15s`
- shutdown is triggered by `SIGINT`, `SIGTERM`, or `SIGHUP`
- logging is initialized through `tracing` before anything else; default filter is `twitch_bot=debug,twitch_sdk=info` and can be overridden with `RUST_LOG`
- EventSub and IRC reconnect after `5s` on any disconnect; EventSub follows `session_reconnect` to the provided `reconnect_url` without re-subscribing
- malformed EventSub messages are logged and skipped instead of dropping the connection
- chat sends check the HTTP status and Helix `is_sent`; on `401` the cached token is invalidated and the send is retried once

### Static dispatch

Every stage is a closed algebraic type, and every router is an exhaustive `match` over it:

- `Event` → `ChatMessage | RewardRedemption | Unsupported`
- `ChatRequest` → `Plain | Command`
- `Command` → `Music | Skip | Unknown(name)`
- `Reward` → `Unknown(id)`

Routers are generic over concrete handler types (`EventRouter<C, R, U>`, `CommandRouter<M, S, U>`, …), so the whole tree is built at compile time and there is no runtime registration or `dyn` dispatch. `Handler` is a native async trait.

The domain is honest about what it does not know: unknown commands and rewards are explicit `Unknown` variants, not a hidden fallback.

Chat text is parsed, not validated: `Command::parse` either produces a `Command` or the message is a plain message. A command name must follow `!` immediately (`! music` is plain text).

### Adding a command

1. add a variant to `Command` and its name to `Command::parse`
2. add a handler field to `CommandRouter` and a `match` arm
3. pass the handler in `bootstrap.rs`

The compiler points at every place that must change.

### Adding a reward

Reward IDs are generated by Twitch when a reward is created in the channel dashboard. Add a variant to `Reward`, map its ID in `Reward::from_id`, then extend `RewardRouter` the same way. A recreated reward gets a new ID and falls into `Reward::Unknown`.

## Event model

The application-level domain event enum is:

- `Event::ChatMessage`
- `Event::RewardRedemption`
- `Event::Unsupported`

The current `twitch-sdk` event model feeding the bot is narrower and only emits:

- `TwitchEvent::ChatMessage`
- `TwitchEvent::RewardRedemption`

Anything unsupported or impossible to map cleanly, including chat events without a complete target, is converted into `Event::Unsupported`.

## Configuration

The bot expects `./config.yaml`.

Example:

```yaml
twitch:
  auth:
    broadcaster_id: "..."
    client_id: "..."
    client_secret: "..."
    refresh_token: "..."
    writer_id: "..."
```

Every field is a non-empty string validated by `#[derive(WrapperType)]` wrappers. Secret fields (`client_secret`, `refresh_token`) use `#[derive(SecretDebug)]`, so they are printed as `***` in `Debug` output.

When Twitch rotates the refresh token, the bot writes the new value back into `config.yaml` atomically and keeps the file permissions. The file is re-serialized by `serde_yaml`, so comments and formatting in it are lost.

## Running

Requirements:

- recent stable Rust with Edition 2024 support
- `playerctl` installed and available in `PATH`
- Unix-like OS
- valid Twitch app credentials and refresh token

Setup:

```bash
cp example.config.yaml config.yaml
```

Run:

```bash
cargo run -p twitch-bot
```

Release build:

```bash
cargo run -p twitch-bot --release
```

## Development

Check the workspace:

```bash
cargo check
```

Run tests:

```bash
cargo test
```

Format:

```bash
cargo fmt
cargo fmt --check
```

Lint:

```bash
cargo clippy --workspace --all-targets
```

Notes:

- `twitch-sdk` has IRC integration tests
- these tests use local sockets and may require a less restricted environment than a sandboxed runner

## Current limitations

These limitations are real in the current code:

- no middleware (role checks, cooldowns) is wired yet
- no rewards are known to the bot yet, every redemption is `Reward::Unknown`
- commands have no role checks or cooldowns yet
- the app is Unix-oriented because it depends on `tokio::signal::unix` and `playerctl`
- unsupported Twitch events are collapsed into `Unsupported` events instead of getting dedicated branches

## License

MIT
