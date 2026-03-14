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

- `!music` calls `playerctl metadata` and sends `сейчас играет трек <artist> - <title>` to chat
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
  -> bootstrap::run
  -> ConfigLoader::load
  -> TokenManager background refresh loop
  -> adapters + routers + handlers wiring
  -> Supervisor::run
  -> EventSource::fetch
  -> Consumer<Event>::consume
  -> EventRouter<Event>
       -> project_chat -> ChatRouter<ChatRequest>
            -> CommandRouter<CommandRequest>
       -> project_reward -> RewardRouter<RewardRequest>
       -> project_system -> SystemHandler
```

The staged dispatch tree is documented visually in:

- `docs/architecture/event-pipeline.png`
- `docs/architecture/event-pipeline.svg`
- `docs/architecture/event-pipeline.mmd`

### Runtime details

The runtime behavior currently implemented in code:

- `Consumer` processes up to `30` events concurrently
- each event handler execution has a `1s` timeout
- graceful shutdown waits up to `10s`
- shutdown is triggered by `SIGINT`, `SIGTERM`, or `SIGHUP`
- logging is initialized through `tracing`

### Routers and typed requests

The dispatch layer uses typed requests per stage:

- `EventRouter<Event>`
- `ChatRouter<ChatRequest>`
- `CommandRouter<CommandRequest>`
- `RewardRouter<RewardRequest>`

The design split is:

- `Router` chooses a branch
- `Projector` converts a broad input into a narrower request
- `Handler` performs business behavior
- `Interceptor` wraps a handler before and/or after execution

In the current codebase:

- `app/dispatch/projector.rs` handles `Event -> ChatRequest | RewardRequest | SystemRequest`
- chat-stage narrowing is completed through `TryFrom<ChatRequest>` into `PlainMessageRequest` or `CommandRequest`
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
environment:
  env: "development"

twitch:
  auth:
    broadcaster_id: "..."
    client_id: "..."
    client_secret: "..."
    access_token: "..."
    refresh_token: "..."
    writer_id: "..."
  bot:
    nick: "..."
    channels:
      - "channel1"
      - "channel2"
    broadcaster_id: "..."
    writer_id: "..."
```

### What is actually used today

The current bootstrap path actively uses:

- `twitch.auth.client_id`
- `twitch.auth.client_secret`
- `twitch.auth.refresh_token`
- `twitch.auth.broadcaster_id`
- `twitch.auth.writer_id`

Important nuance:

- `access_token` exists in the config model, but the current runtime path initializes `TokenManager` from `refresh_token` and refreshes tokens on startup/background loop
- `twitch.bot.*` is currently deserialized, but not used by `bootstrap.rs`

So the schema is a bit ahead of the wiring.

### Validation

Config types use `#[derive(WrapperType)]` wrappers, which gives:

- strongly typed config fields
- string-level validation from the wrapper type
- YAML deserialization into typed wrappers

Current limitation:

- `config::validate::validate` is still a pass-through function
- so cross-field validation and collection-level validation are not implemented yet

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
- `twitch.bot.*` config is not connected to runtime behavior
- config validation beyond wrapper-type checks is not implemented
- the app is Unix-oriented because it depends on `tokio::signal::unix` and `playerctl`
- unsupported Twitch events are collapsed into `System` events instead of getting dedicated branches

## License

MIT
