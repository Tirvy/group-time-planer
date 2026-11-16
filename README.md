# Raid Scheduler

Telegram bot + Telegram Mini App for coordinating recurring weekly group availability preferences. The initial example is organizing a regular World of Warcraft Mythic+ group, but the product is game-agnostic.

## Product in one sentence
A group shares a recurring weekly calendar where each member marks times they **prefer**, sees everyone's individual preferences in their own timezone, and gets a ranked view of popular times.

## Documentation map
- [`docs/01-product-requirements.md`](docs/01-product-requirements.md): scope, terminology, product rules.
- [`docs/02-user-stories-and-acceptance.md`](docs/02-user-stories-and-acceptance.md): user stories and testable acceptance criteria.
- [`docs/03-ux-and-telegram-flows.md`](docs/03-ux-and-telegram-flows.md): bot commands, Mini App screens and interaction flows.
- [`docs/04-architecture.md`](docs/04-architecture.md): stack, components, deployment and technical boundaries.
- [`docs/05-data-model.md`](docs/05-data-model.md): SQLite schema and invariants.
- [`docs/06-api-contract.md`](docs/06-api-contract.md): Mini App API contract.
- [`docs/07-security-and-telegram.md`](docs/07-security-and-telegram.md): Telegram authentication, invitations and operational security.
- [`docs/08-implementation-plan.md`](docs/08-implementation-plan.md): phased delivery plan.
- [`docs/09-decisions-and-open-questions.md`](docs/09-decisions-and-open-questions.md): locked decisions and explicitly deferred features.

## Current MVP boundaries
- Rust backend: `teloxide` + `axum` + `SQLx`.
- SQLite database in project-local `data/app.db`, excluded from Git.
- Vue 3 + Vite + Vuetify frontend.
- Telegram Mini App authentication must be validated server-side.
- Weekly Monday–Sunday calendar, 30-minute resolution.
- Per-user IANA timezone, selected on first calendar opening; same timezone controls preference interpretation and display.
- Display the upcoming Monday–Sunday in the viewer's timezone.
- Maximum 20 members per group.
- Click and drag calendar editing.
- Members can view every member's individual calendar.
- Timetable PNG generated from SVG; `/besttime` ranks preferences, not guaranteed availability.
- No automated database backup feature in MVP.

## Important product language
A selected slot means **preferred**. An unselected slot means **no stated preference**, not unavailable. The product must never label the result as confirmed availability.

## Development

Requirements: Rust 1.90+, Node.js 22.12+.

```bash
# 1. Backend (from the repository root)
cp .env.example .env          # optional; defaults work for local development
cargo run -p raid-scheduler-backend

# 2. Frontend dev server (separate terminal)
cd frontend
npm install
npm run dev                   # http://localhost:5173, proxies /api and /health
```

Production build and single-process run:

```bash
cd frontend && npm run build && cd ..
cargo run -p raid-scheduler-backend --release
# Backend serves frontend/dist on http://127.0.0.1:8080 and exposes GET /health
```

Checks:

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
cd frontend && npm run typecheck && npm run build
```

### Telegram bot
- Put the token from @BotFather into `TELOXIDE_TOKEN` in `.env` or the environment. Without a token the backend runs API-only and logs a warning.
- Implemented commands: `/start`, `/create`, `/groups`, `/invite`, `/members`, `/leave`, `/transfer`, `/delete`, `/help`. All bot text is Russian; command identifiers are English.
- `/create` asks for a group name in a guided step. `/groups` lists memberships and lets the user choose the active group.
- `/invite` (owner only) creates a Telegram deep link. Opening the link shows a read-only preview; joining requires the explicit «Вступить в группу» button, and capacity/membership are revalidated at confirmation. The invitation message has a revoke button.
- `/members` shows the owner first, then other members, with setup status. `/leave`, `/transfer` and `/delete` require confirmation and revalidate permissions server-side.
- The active group and a pending "create" prompt are kept in memory for the single backend instance. After a restart the bot asks the user to choose again instead of relying on stale context.
- The bot token is never logged; configuration redacts it in debug output.

### Telegram Mini App
- The frontend loads the official Telegram Web App SDK and sends the raw `initData` as `Authorization: tma <initData>` on API requests. The backend validates the HMAC signature with the bot token and rejects stale data (`INIT_DATA_MAX_AGE_SECS`, default 24 hours).
- No app session cookie or client-provided user ID is trusted; every protected route resolves identity from validated `initData`.
- API endpoints in this phase: `POST /api/auth/telegram`, `GET /api/me`, `PUT /api/me/timezone`.
- On the first calendar opening the Mini App requires an explicit IANA timezone choice. The same timezone interprets the user's recurring preferences and displays schedules. Changing it later requires a confirmation that explains both effects.
- Outside Telegram the UI shows an "open via Telegram" state; there is no development auth bypass.
- The Mini App requires HTTPS in production. Set the Mini App URL in @BotFather to the HTTPS origin served by the backend.

### Data and operations
- SQLite lives at the project-local `data/app.db`; the directory is created at startup.
- The database file is created only when missing. Startup, migrations and upgrades never delete or recreate it.
- SQLx migrations are embedded in the backend and applied automatically at startup. They are forward-only and never rewrite existing data.
- `data/`, `.env` and SQLite sidecar files are ignored by Git.
- **No automated backups are included in the MVP.** Losing `data/app.db` means losing all groups and preferences permanently.
- Keep the database on a local filesystem, not a network share.
- Telegram Mini Apps require HTTPS in production; terminate TLS in a reverse proxy and forward to the backend.

