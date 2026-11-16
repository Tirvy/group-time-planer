# 04. Architecture

## 1. Stack
- Backend language: Rust.
- Telegram bot: `teloxide`.
- HTTP API: `axum`.
- Database: SQLite via `SQLx`.
- Frontend: Vue 3 + Vite + TypeScript + Vuetify.
- Telegram Mini App integration: official Telegram Web Apps SDK or maintained compatible package.
- Timezone handling: IANA timezone database via `chrono-tz` and a compatible date/time library.
- PNG rendering: generate SVG from backend timetable view model, then convert SVG to PNG (e.g. `resvg`).
- Deployment: one backend process plus static frontend assets; Docker Compose or systemd are both acceptable. Keep initial operational topology simple.

## 2. Process topology
Initially run one Rust backend process that:
- receives Telegram bot updates (long polling is simplest for initial deployment; webhook can be added if deployment requires it),
- serves Mini App API through Axum,
- validates Telegram Mini App authentication,
- owns all domain logic and database access,
- calculates timezone conversions and best-time ranking,
- renders SVG/PNG timetable exports.

Serve the built Vite frontend as static assets from Axum or a lightweight reverse proxy. Do not introduce separate microservices for MVP.

## 3. Frontend rationale
Use Vue 3 + Vite + Vuetify. Nuxt SSR is not required for a Telegram Mini App and would add framework features the MVP does not need. Vuetify is intentionally selected for its cohesive, mature component system and polished UI. Keep bundle size reasonable: import components as needed, lazy-load heavier views, and avoid unnecessary plugins.

## 4. Database
SQLite is appropriate for a small group scheduler with a maximum of 20 members per group and a single backend instance. Use SQLx for migrations, query execution, transactions and compile-time checked queries where practical.

Required SQLite configuration:
```sql
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA busy_timeout = 5000;
```
Apply `foreign_keys` and `busy_timeout` to every connection as appropriate. WAL mode should be initialized and verified. Use a small bounded connection pool and short write transactions. SQLite serializes writers; avoid long-running write transactions and do not run multiple independent backend writers against a shared database file.

Database path: project-local `data/app.db`. Create `data/` automatically. Ignore the entire data directory and SQLite sidecar files in Git. Keep the database on a local filesystem, not a network share.

No automated database backup feature is included in MVP. Document the data-loss consequence in deployment documentation. Do not claim that Git protects the database.

## 5. Suggested repository layout
```text
raid-scheduler/
├── backend/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── config.rs
│       ├── bot/
│       ├── api/
│       ├── auth/
│       ├── domain/
│       ├── services/
│       │   ├── timezone.rs
│       │   ├── preferences.rs
│       │   ├── best_time.rs
│       │   ├── timetable.rs
│       │   └── notifications.rs
│       ├── db/
│       └── errors.rs
├── frontend/
│   ├── package.json
│   ├── vite.config.ts
│   └── src/
│       ├── main.ts
│       ├── plugins/vuetify.ts
│       ├── api/
│       ├── components/
│       ├── views/
│       ├── composables/
│       └── types/
├── migrations/
├── data/                 # ignored by Git; app.db lives here
├── .env.example
├── .gitignore
├── docker-compose.yml
└── README.md
```

## 6. Domain boundaries
Keep domain logic independent of Telegram handlers and HTTP handlers. Both interfaces call the same service layer. Suggested services:
- GroupService: create, rename, list, membership, ownership, deletion.
- InvitationService: issue, preview, revoke, confirm join.
- PreferenceService: validate and replace weekly intervals; setup state.
- TimezoneService: resolve upcoming reference week and convert recurring intervals.
- TimetableService: build a viewer-timezone view model and SVG/PNG.
- BestTimeService: rank converted intervals.
- NotificationService: best-effort owner/member messages.

## 7. Timezone algorithm boundary
Preferences are recurring wall-clock intervals in the member's timezone. For the reference week, expand each member's intervals into actual instants using IANA timezone rules, then convert instants to the viewer's timezone. Use the same service for API, best-time and PNG.

Reference week is the upcoming Monday–Sunday in the viewer's timezone. Define behavior when today is Monday: the current Monday–Sunday week is the upcoming/current week, not the following week. Use a single injected clock abstraction so this is testable.

DST ambiguity/nonexistent local times must follow one documented library-based policy. Do not write a custom timezone engine. Add tests around DST transitions in representative zones. If conversion yields an ambiguous/nonexistent boundary, resolve deterministically and consistently; surface the policy in code comments and tests.

## 8. API and bot consistency
The bot and Mini App must not implement separate business rules. Authorization, group membership, capacity, ownership and preference validation live in backend services and are called from both entry points.

## 9. Deployment and configuration
- Keep secrets in environment variables, not source control.
- `.env` is ignored; commit `.env.example` with placeholders only.
- `data/` is ignored and mounted persistently if Docker is used.
- Ensure application upgrades do not overwrite or recreate the database.
- HTTPS is required for the Telegram Mini App.
- Start with one backend instance.
