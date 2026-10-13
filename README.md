# Mailer

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2024%20edition-orange.svg)](https://www.rust-lang.org/)
[![Poem](https://img.shields.io/badge/Poem-3.1-blueviolet.svg)](https://github.com/poem-web/poem)
[![SeaORM](https://img.shields.io/badge/SeaORM-1.1-green.svg)](https://www.sea-ql.org/SeaORM/)
[![MCP](https://img.shields.io/badge/MCP-Protocol%202024--11--05-brightgreen.svg)](https://modelcontextprotocol.io)

Modern, high-performance, single-binary email marketing and automation platform written in Rust and powered by PostgreSQL.

---

## Key Features

- **Blazing Fast Single Binary**
  - Ultra-low memory footprint and instant startup.
  - No PHP, Node, Python, or external runtime required in production.
  - Structured **JSON logging** powered by `tracing` and `tracing-subscriber`.

- **Modern Architecture & Web Stack**
  - **Web Framework**: [Poem 3.1](https://github.com/poem-web/poem) with cookie sessions, secure CSRF, multipart upload, and internationalization (`i18n`).
  - **Templating & SSR**: [Leptos 0.8](https://leptos.dev) Server-Side Rendering (SSR) styled with Tailwind CSS & Flowbite blocks.
  - **Database & Migrations**: [SeaORM 1.1](https://www.sea-ql.org/SeaORM/) backed by PostgreSQL with automatic schema migration and seed verification on startup.
  - **Asset Pipeline**: Modern ES modules bundled via Vite and Tailwind CSS.
  - **Interactive Template & Content Editor**: Integrated inline rich editor powered by `@templatical/editor` SDK.

- **Responsive MJML Engine**
  - Built-in native [MJML](https://mjml.io) rendering via `mrml`.
  - Write responsive transactional and marketing emails in MJML; the engine compiles them down to battle-tested, client-compatible HTML on the fly while preserving tracking pixels and unsubscribe tags.

- **Sending & Delivery Providers**
  - **Amazon SES SDK v2**: High-throughput sending via AWS SES.
  - **Standard SMTP**: Plain and TLS/STARTTLS SMTP relay (Lettre).
  - **Local Testing**: Built-in compatibility with [Mailpit](https://github.com/axllent/mailpit) for development and offline testing.

- **Bounce & Complaint Handling**
  - Webhook endpoints ready for automated suppression:
    - AWS SNS HTTP notification handler for bounce & complaint events.
    - SendGrid Event Webhook handler for instant bounce suppression.
  - Global suppression lists and blocked domain rules.

- **Comprehensive Tracking & Reports**
  - Transparent 1x1 open tracking pixel.
  - Click-through redirect tracking with campaign and subscriber attribution.
  - Real-time campaign reports with opens, clicks, unsubscribes, bounces, and complaint metrics.

- **Model Context Protocol (MCP) Server**
  - Built-in AI automation endpoint at `/v1/mcp` using `rmcp` Streamable HTTP SSE transport (`2024-11-05` specification).
  - Authenticated via `X-Api-Key` headers.
  - Exposes typed tools across modular categories:
    - `brands`: Inspect, list, and switch brand context.
    - `lists`: Discover recipient lists, manage list configurations, and custom fields.
    - `subscribers`: Search, query status, add, unsubscribe, or delete subscribers.
    - `campaigns`: Draft, preview, send, and review campaign metrics.
    - `templates`: Create, update, and manage email design templates.

- **Type-Safe REST API**
  - Authenticated JSON REST endpoints under `/api`.
  - AES-CBC encrypted `OpaqueId` tokens preventing sequential ID enumeration attacks.

---

## Quick Start

### Prerequisites
- **Rust**: 1.85+ (2024 edition)
- **Node.js**: v20+ & npm (for bundling assets)
- **PostgreSQL**: 14+ (or Docker)

### 1. Clone & Setup Configuration
```bash
git clone https://github.com/wearekesk/mailer.git
cd mailer

cp .env.example .env
```

Review `.env` to configure your database URL and secrets:
```dotenv
BIND_ADDR=0.0.0.0:3000
APP_URL=http://localhost:3000
DATABASE_URL=postgres://mailer:mailerpass@localhost:5432/mailer
SECRET_KEY=change-me-in-production-mailer-secret-key-32chars!
MAILER_ADMIN_EMAIL=admin@mailer.local
MAILER_ADMIN_PASSWORD=ChangeMe!2026
```

### 2. Build Assets & Run
```bash
# Install dependencies & build frontend bundle
npm install
npm run build

# Start the Mailer server (runs migrations automatically)
cargo run --bin mailer
```
Open [http://localhost:3000](http://localhost:3000) and sign in with the configured admin credentials.

---

## Docker Compose Quick Start

To launch Mailer along with PostgreSQL and Mailpit in one command:

```bash
docker compose up -d
```

- **Mailer Web UI**: [http://localhost:3000](http://localhost:3000)
- **Mailpit Web UI**: [http://localhost:8025](http://localhost:8025)
- **PostgreSQL**: `localhost:5432`

---

## Model Context Protocol (MCP) Integration

Mailer implements the Model Context Protocol to allow AI coding assistants and automation workflows (e.g. Claude Desktop, Cursor, Gemini CLI) to manage email operations programmatically.

To connect your MCP client:
- **URL**: `http://localhost:3000/v1/mcp`
- **Transport**: `streamable-http` (SSE)
- **Headers**: `X-Api-Key: <your-api-key>`

---

## Testing

```bash
# Run unit and integration tests
cargo test

# Run MCP test suite (with server running)
python3 tests/test_all_mcp_tools.py
```

---

## License

This project is licensed under the [MIT License](LICENSE).
