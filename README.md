# Twitch Bot

Twitch bot workspace in Rust with a small staged event pipeline:

- Twitch EventSub is the input source
- incoming SDK events are mapped into domain `Event`
- `Event` is projected into typed requests (`ChatRequest`, `CommandRequest`, `RewardRequest`, `SystemRequest`)
- routers dispatch by stage
- handlers execute bot behavior

The current executable lives in `crates/twitch-bot`. The workspace also contains a small Twitch SDK and config helper macros.

## Current behavior

What the bot actually does today:

- consumes Twitch EventSub events through `twitch-sdk`
- supports two domain event kinds in the main pipeline:
  - chat messages
  - channel point reward redemptions
- converts unsupported SDK events and chat messages without a complete target into `System` events
- routes chat messages into:
  - plain messages
  - commands
- supports two registered commands:
  - `!music`
  - `!skip`
- logs and ignores unknown commands
- logs reward redemptions through a fallback reward handler
- logs plain chat messages at `trace`
- logs system/fallback events at `warn`

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
│   ├── command/    # command parsing and typed command names
│   ├── dispatch/   # handlers, interceptors, routers, typed requests
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
       -> SystemRequest -> SystemHandler
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

### Routers and typed requests

The dispatch layer uses typed requests per stage:

- `EventRouter<Event>`
- `ChatRouter<ChatRequest>`
- `CommandRouter<CommandRequest>`
- `RewardRouter<RewardRequest>`

The design split is:

- `Router` chooses a branch and converts a broad input into a narrower request via `From`
- `Handler` performs business behavior
- `Interceptor` wraps a handler before and/or after execution

In the current codebase:

- `EventRouter` matches on `Event` and converts each variant into `ChatRequest | RewardRequest | SystemRequest`
- `ChatRouter` matches on `ChatRequest::Plain | ChatRequest::Command`
- interceptors are supported by the builders, but no concrete interceptors are wired in `bootstrap` yet

### Dynamic route registration

Two route spaces are intentionally runtime-driven:

- commands are registered by `CommandName`
- reward handlers are registered by `RewardId`

That means commands and rewards are not modeled as closed enums.

Current command registration happens in `bootstrap.rs` through:

```rust
CommandRouter::builder()
    .route("music", Arc::new(MusicHandler::new(...)))
    .route("skip", Arc::new(SkipHandler::new(...)))
    .fallback(Arc::new(UnknownCommandHandler::new()))
    .build()?;
```

Reward routing follows the same model:

```rust
RewardRouter::builder()
    .route("reward-id", Arc::new(MyRewardHandler))
    .fallback(Arc::new(RewardRedemptionHandler::new()))
    .build()?;
```

## Event model

The application-level domain event enum is:

- `Event::ChatMessage`
- `Event::RewardRedemption`
- `Event::System`

The current `twitch-sdk` event model feeding the bot is narrower and only emits:

- `TwitchEvent::ChatMessage`
- `TwitchEvent::RewardRedemption`

Anything unsupported or impossible to map cleanly, including chat events without a complete target, is converted into `Event::System`.

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

- no concrete interceptors are wired yet
- reward routing only has a fallback handler in `bootstrap`
- commands have no role checks or cooldowns yet
- the app is Unix-oriented because it depends on `tokio::signal::unix` and `playerctl`
- unsupported Twitch events are collapsed into `System` events instead of getting dedicated branches

## License

MIT
