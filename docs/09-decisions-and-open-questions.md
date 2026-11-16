# 09. Decisions and Open Questions

## 1. Purpose and precedence

This document records decisions that are binding for the MVP but are not fully
obvious from a single requirements page. It also lists questions that are still
open and features that are explicitly deferred.

Rules:
- A **locked** decision may only be changed by the product owner.
- If another document contradicts a locked decision, this document wins for the
  specific point and the other document should be corrected.
- If a requirement is neither specified elsewhere nor listed here as open or
  deferred, stop and ask the product owner. Do not invent behavior.
- Dates use ISO `YYYY-MM-DD`.

## 2. Locked product decisions

### 2.1 Preference semantics
- A selected slot means **preferred**.
- An unselected slot means **no stated preference**, never "unavailable" or
  "busy". Blank timetable space is not proof of unavailability.
- `setup_pending` means `setup_completed_at IS NULL` (no successful save yet).
- `setup_completed` means at least one save succeeded, including a save with
  **zero** intervals. Empty means "intentionally no preferred times".
- Pending members are excluded from best-time counts and denominators.
  Completed members with zero intervals count as configured and contribute zero
  preferences.
- Best-time output is a preference ranking, never confirmed availability or an
  automatic booking recommendation.

### 2.2 Calendar model
- Recurring weekly calendar, Monday through Sunday.
- 48 half-hour slots per day, 336 slots per week.
- Preferences are local `weekday` (Monday = 1 … Sunday = 7) plus minute
  boundaries, half-open `[start, end)`.
- Boundaries are multiples of 30 minutes; `0 <= start_minute <= 1439`,
  `1 <= end_minute <= 1440`, `start_minute < end_minute`.
- Intervals for the same group, member and weekday must not overlap. Adjacent
  intervals are allowed and the UI may merge them for display.
- Saving replaces all intervals for the authenticated member in the active group.

### 2.3 Timezones and the reference week
- Each user selects one IANA timezone explicitly on first calendar opening. The
  UI may suggest the device timezone but must not save it without confirmation.
- The same timezone governs both interpretation of the user's recurring local
  preferences and display of schedules to that user.
- Changing the timezone changes interpretation and display together; the user
  must be warned before changing it.
- No calendar, ranking or PNG view may silently fall back to the server timezone.
- Store IANA identifiers (for example `Asia/Tbilisi`), never fixed UTC offsets,
  and never store recurring preferences as UTC timestamps.
- The reference week is the upcoming Monday–Sunday in the **viewer's** timezone.
  If the current local day is Monday, the week starts today (current week, not
  next week).
- Each member's recurring local intervals are expanded into actual instants for
  that reference week using IANA rules, then converted to the viewer's timezone.
  Raw weekday/minute values must never be compared across timezones for ranking.
- One shared backend conversion module serves the interactive API, `/besttime`
  and PNG rendering. The bot and Mini App must not implement separate rules.
- Show the reference-week date range wherever a timetable or ranking is shown.
- Use a maintained library (`chrono-tz` with `chrono`); do not build a timezone
  engine.

### 2.4 Groups, roles and membership
- Maximum 20 members per group, including the owner.
- The creator becomes owner and first member in one transaction.
- Roles are only `owner` and `member`.
- The owner cannot leave while still owner. They must transfer ownership to an
  existing member or delete the group.
- Ownership transfer keeps the previous owner as a regular member.
- There is exactly one owner per group, consistent between the group record and
  membership roles.
- Owner capabilities: rename group, manage invitations, transfer ownership,
  delete group, view all schedules.
- Member capabilities: edit own preferences, view all group calendars, leave.
- Every member can view every member's individual calendar; other members'
  calendars are read-only and the viewer's own calendar is visually distinct.
- Each membership has a stable color within that group.
- Preferences are scoped per group. Leaving group A must not affect group B.
  The user profile survives leaving a group.

### 2.5 Invitations
- Invitation links use Telegram deep linking:
  `https://t.me/<BOT_USERNAME>?start=join_<TOKEN>`.
- Opening a link shows a read-only preview and never creates membership.
- The user must press an explicit Russian confirmation action to join.
- Confirmation revalidates, inside the join transaction: token hash, expiry,
  revocation, group existence, capacity and existing membership.
- Joining is idempotent; repeated confirmation never creates duplicates.
- Tokens have at least 128 bits of entropy (256 bits by default), are URL-safe
  opaque values, and only a cryptographic hash is stored.
- Invitations support revocation and optional expiration.

### 2.6 Notifications
- Owner notifications are best effort: new member joined, member left, member
  saved changed preferences.
- A failed notification must never roll back a successful domain write.
- Telegram bots cannot start private chats with users who never started the bot.
  The limitation is explained to users and delivery failures are logged safely.
- After ownership transfer, subsequent owner notifications target the new owner.

### 2.7 Localization
- All user-facing bot text, buttons, notifications and Mini App text are Russian.
- Telegram command identifiers stay English: `/create`, `/groups`, `/invite`,
  `/calendar`, `/timetable`, `/besttime`, `/members`, `/leave`, `/transfer`,
  `/delete`, `/help`.
- User-facing strings are centralized (bot message module and frontend
  `src/i18n/ru.ts`), not scattered through handlers or components.

### 2.8 Data, persistence and operations
- The SQLite database lives at project-local `data/app.db`.
- The database must stay on a local filesystem, never a network share.
- `data/`, `.env`, `*.db`, `*.db-wal` and `*.db-shm` are ignored by Git.
- Startup, migration and deployment must never delete or recreate the database.
- **No automated database backups exist in the MVP.** Losing `data/app.db` means
  permanent data loss. Git does not protect the database.
- Run one backend instance for the MVP.

### 2.9 Security
- Validate Telegram Mini App `initData` server-side (signature and freshness);
  never trust `initDataUnsafe` or a client-supplied Telegram user ID.
- Serialize Telegram IDs as strings in JSON/JavaScript to avoid precision loss.
- Enforce membership and owner checks on every backend route.
- Use transactions for join/capacity, preference replacement, ownership
  transfer and deletion-related state changes.
- Keep secrets in environment variables; never log the bot token, raw
  `initData`, raw invitation tokens or session secrets.
- Use parameterized SQL, validate all input on the backend, set request size
  limits, and serve the Mini App over HTTPS in production.

## 3. Locked technical decisions

| Area | Decision | Date |
|---|---|---|
| Backend | Rust, one process serving Telegram updates and HTTP API | 2026-11-16 |
| Bot | `teloxide`, long polling initially (webhook may be added later) | 2026-11-16 |
| HTTP | `axum` | 2026-11-16 |
| Database | SQLite via `SQLx`, WAL, foreign keys on every connection, busy timeout 5 s, `synchronous=NORMAL`, bounded pool (1–16, default 5), short write transactions | 2026-11-16 |
| Frontend | Vue 3 + Vite + TypeScript + **Vuetify 4.x**; no Nuxt SSR, no other component library | 2026-11-16 |
| Timezones | `chrono` + `chrono-tz` | 2026-11-16 |
| PNG | Backend SVG view model converted with `resvg` | 2026-11-16 |
| Process topology | One backend process; static Vite build served by Axum with SPA fallback | 2026-11-16 |
| Configuration | Environment variables, optional `.env`; defaults: `127.0.0.1:8080`, `data/app.db`, `frontend/dist`, pool 5 | 2026-11-16 |
| Logging | `tracing` + `EnvFilter`; optional `LOG_FORMAT=json`; no secrets in logs | 2026-11-16 |
| Health | `GET /health`: `200` when usable, `503` when the database fails or times out (2 s); JSON with service/version, no internals | 2026-11-16 |
| Storage | Internal primary keys are positive `INTEGER`; timestamps are UTC milliseconds since the Unix epoch | 2026-11-16 |
| Migrations | Embedded SQLx migrations, forward-only, applied automatically at startup; existing data is never recreated | 2026-11-16 |
| Write transactions | `BEGIN IMMEDIATE` and short transactions; group creation and membership joins are transactional | 2026-11-16 |
| Database invariants | Triggers enforce the 20-member cap and non-overlapping preferences in addition to service checks | 2026-11-16 |
| Bot | `teloxide` 0.17 with long polling and rustls; no webhook endpoint in the MVP | 2026-11-16 |
| Bot context | Active group selection and pending group-name prompt are in memory per user (10-minute TTL) and clear on restart | 2026-11-16 |
| Bot security | Token is held in a redacting wrapper; `tracing-log` is intentionally disabled so token-bearing HTTP client logs are dropped | 2026-11-16 |
| Invitations | 32 random bytes encoded as base64url (43 chars); only a SHA-256 hex hash is stored; preview is read-only; join is transactional and idempotent; owner can revoke | 2026-11-16 |
| Notifications | Sent only after a successful commit, with a 3-second timeout; failures are logged without error details and never roll back domain writes | 2026-11-16 |
| Mini App auth | Per-request `Authorization: tma <initData>` validated with HMAC-SHA256 using the bot token; no app session cookie or bearer token in the MVP | 2026-11-16 |
| initData freshness | Default 24 hours (`INIT_DATA_MAX_AGE_SECS`), allowed 60 seconds to 7 days; up to 60 seconds of future clock skew | 2026-11-16 |
| API errors | Stable envelope `{ "error": { "code", "message" } }`; the frontend maps codes to centralized Russian copy | 2026-11-16 |
| Timezone UI | Browser IANA list via `Intl.supportedValuesOf` with a static fallback; explicit save; backend revalidates with `chrono-tz` | 2026-11-16 |
| Preferences API | Local `(weekday, start_minute, end_minute)` intervals; `PUT` replaces atomically; empty list is valid and marks setup completed; `setup_completed_at` records the first successful save | 2026-11-16 |
| Calendar editor | Monday–Sunday 30-minute grid; tap toggles; pointer drag paints; vertical touch scrolling is preserved; keyboard activation is supported | 2026-11-16 |
| Preference notifications | Owner is notified after commit when a non-owner's normalized intervals change; delivery failures never roll back the save | 2026-11-16 |
| Group summary | Includes viewer role, stable membership color, member count and setup state | 2026-11-16 |
| Reference week | Upcoming Monday–Sunday in the viewer's IANA timezone; Monday today starts the current week; stored as viewer-local dates plus UTC instants | 2026-11-16 |
| Conversion | Recurring local intervals are expanded to reference-week instants, clipped to the week, then split at viewer midnight and viewer DST transitions | 2026-11-16 |
| Combined read grid | 15-minute slices in the viewer timezone so offset conversions such as :15/:45 remain accurate; each slice shows member color dots and is inspectable | 2026-11-16 |
| Clock | A single injected `Clock` abstraction supplies "now" to reference-week calculation; tests use a fixed clock | 2026-11-16 |
| Telegram IDs | Stored internally as signed 64-bit integers, exposed as JSON strings | 2026-11-16 |
| Group name | Trimmed, non-empty, **1–64 characters** | 2026-11-16 |
| Russian copy | Centralized; no scattered literals in components or handlers | 2026-11-16 |

### 3.1 DST conversion policy

The policy is library-based and deterministic. It is applied to both interval
boundaries, and it is identical for the API, `/besttime` and PNG rendering.

- **Ambiguous local time** (fall-back, the wall clock repeats): choose the
  **earlier UTC offset**, i.e. the first occurrence.
- **Nonexistent local time** (spring-forward gap): advance the wall-clock value
  by the size of the gap. This preserves interval duration. Example under a
  one-hour gap: `02:30` resolves to `03:30`; an interval `[02:00, 03:00)`
  resolves to `[03:00, 04:00)`.
- Normal times use the library's ordinary IANA resolution.
- Tests must cover representative zones and both transitions (for example
  `Europe/Berlin` and a zone with a non-one-hour or differently timed change)
  before Phase 6 is considered done.

## 4. Open questions

These are not locked; do not silently pick permanent behavior. Interim choices
are implementation details that are cheap to change and must be revisited before
the related phase is marked done.

| # | Question | Target phase | Interim behavior |
|---|---|---|---|
| 1 | Exact rate limits and mechanism for invitation preview/join and PNG generation | 7–8 | Request size limits and short transactions; single instance |
| 2 | Exact 20-color palette and assignment rule for stable member colors | 1 | Deterministic distinct palette assigned on join; proposal, confirm before polish |
| 3 | Group primary key type: integer or UUID | 1 | Integer primary key implemented in migration `0001`; changing it requires a migration |
| 4 | Notification retry policy | post-MVP | Log failures only; never retry within the request |
| 5 | Whether percentages are displayed anywhere | 5–7 | Optional; if shown, denominator = completed members |
| 6 | Cleanup of expired/revoked invitation rows | post-MVP | Keep rows; validation rejects them |
| 7 | Deployment target: Docker Compose or systemd | 8 | Both acceptable per architecture; document the chosen one |
| 8 | Should the bot remember the active group across restarts? | post-MVP | In-memory selection; the user is asked to choose again after a restart |
| 9 | What should the default invitation lifetime be? | post-MVP | No expiry by default; the owner can revoke the invitation at any time |

## 5. Explicitly out of scope

No WoW API integration, character or role management, dungeon readiness checks,
automatic event booking, attendance tracking, reminders, recurring event
creation, hard-unavailable/busy slots, moderator roles, calendar integrations,
historical analytics, PostgreSQL, Redis, message brokers, microservices,
Kubernetes, Nuxt SSR, or **automated database backups**.

## 6. Change log

| Date | Change |
|---|---|
| 2026-11-16 | Repaired documentation numbering (files were offset by one and `02` duplicated `01`). Created this decisions document. Locked Vuetify 4.x, group name length 1–64, and the DST policy above. |
| 2026-11-16 | Phase 1: added the initial SQLite schema (`users`, `groups`, `group_members`, `weekly_preferences`, `invitations`), domain types, transactional group/membership services, and migration/constraint/cascade tests. Recorded storage, migration and transaction decisions above. |
| 2026-11-16 | Phase 2: added the Telegram bot (`/start`, `/create`, `/groups`, `/help`), user upsert from Telegram updates, group creation and selection, Russian bot copy, redacted token configuration, and service tests. |
| 2026-11-16 | Phase 3: added secure invitations with preview, explicit join confirmation, revocation and expiry checks; members list; leave, ownership transfer, rename and deletion with notifications. Added lifecycle and invitation tests. |
| 2026-11-16 | Phase 4: added server-side Telegram `initData` validation, authenticated API context, `/api/me`, timezone persistence, and the Mini App shell with mandatory timezone setup and change confirmation. Resolved the session-strategy question in favour of per-request validation. |
| 2026-11-16 | Phase 5: added the preference model and transactional replacement service, group list and preference API endpoints, the Monday–Sunday 30-minute editor, tap/drag/keyboard interaction, clear/discard/save flows, and pending vs completed-empty states. Added backend and frontend model tests. |
| 2026-11-16 | Phase 6: added the shared reference-week and timezone conversion service with the documented DST policy, combined and individual calendar APIs, converted viewer-timezone read views, and DST/reference-week tests. |
