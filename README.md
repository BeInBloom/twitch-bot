# Twitch Bot

![Rust](https://img.shields.io/badge/Rust-1.93.1-orange.svg)
![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Architecture](https://img.shields.io/badge/architecture-DDD-green.svg)

Модерный Twitch бот на Rust с чистой архитектурой и поддержкой EventSub API.

---

## 📋 О проекте

Это Twitch бот, построенный с использованием Domain-Driven Design (DDD) и принципов чистой архитектуры. Проект разработан с акцентом на расширяемость, безопасность типов и надежность.

**Ключевые особенности:**

- 🎯 EventSub/WebSocket интеграция для работы в реальном времени
- 🏗️ Чистая архитектура с четким разделением слоев
- 🔒 Типизированная конфигурация с автоматической валидацией
- 🛡️ Graceful shutdown с CancellationToken
- 📦 Модульная система роутинга событий
- 🔄 Автоматическое обновление OAuth токенов
- ⚡ Высокая производительность на async/await

---

## 🏗️ Архитектура проекта

### Общая схема

```
┌─────────────────────────────────────────────────────────┐
│                    APPLICATION ENTRY                    │
│                      (main.rs)                          │
└────────────────────┬────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────┐
│                   CORE LAYER                            │
│              (Оркестрация приложения)                   │
│  ┌──────────┐  ┌──────────┐  ┌──────────────────┐       │
│  │    App   │  │ Shutdown │  │  SignalHandler   │       │
│  └──────────┘  └──────────┘  └──────────────────┘       │
└────────────────────┬────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────┐
│                 DOMAIN LAYER                            │
│             (Бизнес-логика и модели)                    │
│  ┌──────────┐  ┌──────────┐  ┌──────────────────┐       │
│  │  Models  │  │  Traits  │  │   Errors         │       │
│  │ (Event,  │  │(Fetcher, │  │ (ConfigError)    │       │
│  │  User)   │  │Consumer) │  │                  │       │
│  └──────────┘  └──────────┘  └──────────────────┘       │
└────────────────────┬────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────┐
│               INFRASTRUCTURE LAYER                      │
│          (Реализация внешних зависимостей)              │
│  ┌──────────┐  ┌──────────┐  ┌──────────────────┐       │
│  │ Config   │  │  Router  │  │   Sender         │       │
│  │ (Loader, │  │ (Handler │  │ (TwitchSender)   │       │
│  │  Models) │  │  chain)  │  │                  │       │
│  └──────────┘  └──────────┘  └──────────────────┘       │
│  ┌──────────┐  ┌──────────┐  ┌──────────────────┐       │
│  │ Fetcher  │  │ Consumer │  │   Logging        │       │
│  │(Twitch)  │  │ (Buffer  │  │                  │       │
│  │          │  │ control) │  │                  │       │
│  └──────────┘  └──────────┘  └──────────────────┘       │
└────────────────────┬────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────┐
│               EXTERNAL DEPENDENCIES                     │
│  ┌──────────┐  ┌──────────┐  ┌──────────────────┐       │
│  │ Twitch   │  │   YAML   │  │   File System    │       │
│  │   API    │  │  Config  │  │                  │       │
│  └──────────┘  └──────────┘  └──────────────────┘       │
└─────────────────────────────────────────────────────────┘
```

### Описание слоев

#### 🎯 Core Layer (`src/core/`)

**Отвечает за оркестрацию приложения**

Компоненты:

- **App** - главный оркестратор, объединяет fetcher, consumer и signal handler
- **Shutdowner** - трейт для graceful shutdown
- **SignalHandler** - обработка системных сигналов (SIGINT, SIGTERM)

Ключевой принцип: Core слой знает "когда" делать, но не "что" делать.

#### 🧠 Domain Layer (`src/domain/`)

**Содержит бизнес-логику и не зависит от внешних систем**

Компоненты:

- **Models** - доменные модели (`Event`, `User`, `Role`, `Platform`)
- **Traits** - абстракции для внешних зависимостей:
  - `EventFetcher` - интерфейс для получения событий
  - `EventConsumer` - интерфейс для обработки событий
  - `Sender` - интерфейс для отправки сообщений
- **Errors** - специфичные ошибки домена

Ключевой принцип: Domain слой полностью независим от инфраструктуры.

#### 🔧 Infrastructure Layer (`src/infra/`)

**Реализует интерфейсы домена и работает с внешними системами**

Компоненты:

- **Config** - загрузка и валидация конфигурации из YAML
- **Router** - маршрутизация событий по типам (Message, Command, etc.)
- **Fetcher** - реализация `EventFetcher` (Twitch EventSub)
- **Consumer** - реализация `EventConsumer` с backpressure control
- **Sender** - реализация `Sender` (Twitch Helix API)
- **Logging** - настройка tracing

Ключевой принцип: Infra слой реализует интерфейсы, определенные в Domain.

---

## 📦 Структура крейтов

```
crates/
├── macros/              # Proc-macro для #[derive(WrapperType)]
│   └── src/lib.rs      # Генерация TryFrom, Deserialize, as_str()
├── macros-core/         # Общие типы для макросов
│   └── src/
│       ├── lib.rs       # Re-export WrapperValidationError
│       └── errors.rs    # Тип ошибки валидации
├── twitch-sdk/         # SDK для Twitch API
│   └── src/
│       ├── auth.rs      # TokenManager для OAuth
│       ├── chat/        # IRC клиент и Helix Sender
│       ├── eventsub/    # EventSub WebSocket клиент
│       ├── irc/         # IRC парсер и клиент
│       └── types.rs     # TwitchEvent, TwitchUser, TwitchRole
└── twitch-bot/         # Основное приложение
    └── src/
        ├── core/        # Оркестрация, shutdown, сигналы
        ├── domain/      # Модели, трейты, ошибки
        └── infra/       # Конфигурация, router, fetcher, sender
```

### Twitch SDK

Отдельный крейт, который можно использовать независимо:

- **EventSubClient** - WebSocket клиент для Twitch событий
- **IrcClient** - IRC клиент для чата (опционально)
- **TokenManager** - автоматическое обновление токенов
- Типы: `TwitchEvent`, `TwitchUser`, `TwitchRole`

---

## 🚀 Быстрый старт

### 1. Клонирование

```bash
git clone <repo-url>
cd twitch-bot
```

### 2. Настройка конфигурации

Скопируйте пример конфигурации:

```bash
cp example.config.yaml config.yaml
```

### 3. Получение Twitch креденшалов

1. Перейдите на [Twitch Developers Console](https://dev.twitch.tv/console)
2. Создайте приложение (Applications → Register Your Application)
3. Получите `client_id` и `client_secret`
4. Используйте OAuth Authorization Code flow для получения токенов
5. Узнайте ваш `user_id` (используется как broadcaster_id и writer_id)

### 4. Запуск

```bash
# Debug версия
cargo run

# Release версия (оптимизированная)
cargo run --release
```

---

## ⚙️ Конфигурация

### Структура config.yaml

```yaml
environment:
  env: "development" # "development" | "production" | "staging"

twitch:
  auth:
    client_id: "your_client_id"
    client_secret: "your_client_secret"
    access_token: "your_access_token"
    refresh_token: "your_refresh_token"
    broadcaster_id: "broadcaster_user_id"
    writer_id: "bot_user_id"
  bot:
    nick: "bot_name"
    channels:
      - "channel1"
      - "channel2"
    broadcaster_id: "broadcaster_user_id"
    writer_id: "bot_user_id"
```

### Типы полей

- **client_id, client_secret** - из Twitch Developers Console
- **access_token, refresh_token** - получаются через OAuth flow
- **broadcaster_id** - ID канала, на котором работает бот
- **writer_id** - ID пользователя, от имени которого бот пишет сообщения
- **nick** - имя бота для подключения
- **channels** - список каналов для подключения

---

## 📖 Использование

### Поток данных

```
Twitch EventSub
       │
       ▼
TwitchFetcher (implements EventFetcher)
       │
       ▼
   Event
       │
       ▼
BaseRouter (determines Route)
       │
       ▼
  Handler (MessageHandler, CommandHandler, etc.)
       │
       ▼
  Processing
       │
       ▼
Sender (implements Sender)
       │
       ▼
Twitch Helix API
```

### Создание custom handler

Создайте новый handler, реализующий трейт `Handler`:

```rust
use async_trait::async_trait;
use crate::domain::models::{Event, EventKind};
use crate::infra::consumer::router::traits::Handler;

pub struct CustomHandler;

#[async_trait]
impl Handler for CustomHandler {
    async fn handle(&self, event: Event) -> anyhow::Result<()> {
        match &event.kind {
            EventKind::ChatMessage { text } => {
                println!("Received message: {}", text);
                // Обработка сообщения
            }
            EventKind::Command { name, args } => {
                println!("Command: {} with args: {:?}", name, args);
                // Обработка команды
            }
            _ => {}
        }
        Ok(())
    }
}
```

### Добавление нового роута

Добавьте новый вариант в `Route` enum и зарегистрируйте handler:

```rust
// 1. Добавьте новый вариант Route
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Route {
    Message,
    Command,
    MyCustomEvent,  // ← новый вариант
}

// 2. Обновите From<&Event> для Route
impl From<&Event> for Route {
    fn from(event: &Event) -> Self {
        match &event.kind {
            EventKind::ChatMessage { .. } => Route::Message,
            EventKind::Command { .. } => Route::Command,
            // Добавьте логику для вашего события
            _ => Route::MyCustomEvent,
        }
    }
}

// 3. Зарегистрируйте handler в main.rs
let router = BaseRouter::new()
    .route(Route::Message, Arc::new(MessageHandler::new(sender)))
    .route(Route::Command, Arc::new(CommandHandler::new()))
    .route(Route::MyCustomEvent, Arc::new(CustomHandler::new()));
```

### Использование Sender

```rust
use crate::domain::sender::Sender;

async fn send_greeting(sender: &Arc<dyn Sender>) -> anyhow::Result<()> {
    sender.send("channel_id", "Hello from bot!").await
}
```

---

## 🔧 Разработка

### Требования

- Rust 1.93.1 или выше
- Cargo 2024 edition

### Запуск тестов

```bash
# Все тесты
cargo test

# Только unit тесты
cargo test --lib

# Интеграционные тесты (требуют mock серверов)
cargo test --test irc_client_tests

# Тесты с выходом в stdout
cargo test -- --nocapture
```

### Linting и форматирование

```bash
# Проверка clippy
cargo clippy --all-targets --all-features

# Авто-исправление clippy
cargo clippy --fix --allow-dirty --allow-staged

# Проверка форматирования
cargo fmt --check

# Авто-форматирование
cargo fmt
```

### Сборка

```bash
# Debug сборка (быстрая, без оптимизаций)
cargo build

# Release сборка (оптимизированная)
cargo build --release
```

### Добавление новой функциональности

1. **Domain слой**: Добавьте модель или трейт
2. **Infra слой**: Реализуйте интерфейс домена
3. **Core слой**: Интегрируйте в оркестрацию
4. **Тесты**: Покройте новую функциональность тестами

---

## 🔐 Система конфигурации

### Типизированная конфигурация

Проект использует proc-macro `#[derive(WrapperType)]` для создания типобезопасных wrapper types:

```rust
#[derive(WrapperType)]
pub struct ClientId(String);

// Автоматически генерирует:
// - impl TryFrom<String> for ClientId с валидацией
// - impl Deserialize for ClientId
// - метод as_str() для доступа к значению
```

### Преимущества

1. **Безопасность типов**: невозможно перепутать `client_id` с `broadcaster_id`
2. **Автоматическая валидация**: проверка на пустые строки при десериализации
3. **Удобный API**: доступ к значению через `as_str()` метод
4. **Поддержка YAML**: нативная десериализация из YAML файлов

### Валидация

Все поля конфигурации автоматически проверяются при загрузке:

- Строки не могут быть пустыми
- Массивы не могут быть пустыми (например, `channels`)
- Ошибки валидации возвращаются как `ConfigError`

---

## 🚦 Lifecycle приложения

### Запуск

1. Загрузка конфигурации (`ConfigLoader::load()`)
2. Инициализация logging (`LogGuard::init()`)
3. Создание компонентов (`Sender`, `Router`, `Consumer`, `Fetcher`)
4. Создание `App` и запуск (`app.run()`)

### Работа

1. `Fetcher` получает события из Twitch EventSub
2. `Consumer` обрабатывает события в цикле
3. `Router` определяет `Route` для каждого события
4. `Handler` обрабатывает событие
5. `Sender` отправляет сообщения в Twitch

### Graceful Shutdown

1. `SignalHandler` ловит SIGINT/SIGTERM/SIGHUP
2. `Fetcher.shutdown()` останавливает WebSocket соединение
3. `Consumer` завершает обработку событий (с таймаутом 10s)
4. Приложение завершает работу

---

## 🗺️ Roadmap

### Планируется

- [ ] Поддержка middleware в роутере (логирование, метрики, аутентификация)
- [ ] Database слой для сохранения состояния
- [ ] Команды с аргументами и флагами
- [ ] Rate limiting для защиты от спама
- [ ] Модульная система плагинов
- [ ] Web dashboard для управления
- [ ] Логирование всех событий в базу данных
- [ ] Поддержка несколько платформ (YouTube, Discord)

---

## 📄 License

Этот проект распространяется под лицензией MIT. Подробности в файле LICENSE.

---

## 📞 Контакты

Если у вас есть вопросы или предложения, пожалуйста:

- Откройте issue на GitHub
- Напишите в обсуждениях
- Свяжитесь с автором
