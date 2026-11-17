# 08. Implementation Plan

Build vertical slices. Keep the application runnable at the end of each phase. Do not generate the entire codebase in one unreviewed pass.

## Phase 0: Repository and tooling
- Create Rust backend and Vue 3 + Vite + TypeScript + Vuetify frontend.
- Add formatting/linting and basic CI checks if desired.
- Add configuration loading and `.env.example`.
- Add `.gitignore` for `.env`, `data/`, SQLite database and WAL/SHM files.
- Create `data/` automatically at runtime.
- Establish a health endpoint and structured logging.

Acceptance: frontend builds; backend starts; health endpoint responds; local SQLite file is created under `data/`; database is not tracked by Git.

## Phase 1: SQLite and domain foundation
- Add SQLx and migrations.
- Configure SQLite foreign keys, WAL and busy timeout.
- Implement users, groups, memberships, preferences and invitations tables.
- Implement service layer and transaction helpers.
- Add tests for constraints and cascades.

Acceptance: migrations run on clean DB; group membership uniqueness and 20-member cap are tested; deleting membership only removes that group's preferences.

## Phase 2: Telegram bot and identity
- Implement bot startup and `/start`.
- Implement user upsert from Telegram bot updates.
- Implement `/create`, `/groups`, `/help` and group selection.
- Add group creation flow and creator-as-first-member behavior.

Acceptance: user can start bot, create group, and list/select their groups.

## Phase 3: Invitations and lifecycle
- Implement secure invitation creation, preview and explicit join confirmation.
- Implement revoke/expiration.
- Implement `/members`, `/leave`, `/transfer` and `/delete`.
- Implement best-effort owner notifications.

Acceptance: lifecycle acceptance criteria pass, including race-safe capacity, transfer, leave restrictions and deletion cascades.

## Phase 4: Mini App authentication and timezone setup
- Integrate Telegram Web Apps SDK.
- Validate `initData` server-side.
- Add authenticated API session/context.
- Build Vuetify shell and timezone selection screen.
- Persist IANA timezone; require explicit confirmation on first calendar visit.

Acceptance: tampered/stale initData rejected; client-supplied user ID cannot impersonate another user; calendar is blocked until timezone set.

## Phase 5: Preference calendar editor
- Build Monday–Sunday, 30-minute grid.
- Implement click toggle and drag range selection.
- Implement clear, save, discard and unsaved-change warning.
- Implement backend interval validation and transactional replacement.
- Distinguish pending from completed-empty.

Acceptance: all preference validation and setup-state tests pass; mobile scroll and touch interactions work.

## Phase 6: Timezone conversion and read views
- Implement shared reference-week calculation.
- Implement timezone conversion service using IANA rules.
- Build combined timetable and individual member calendar view.
- All members can inspect all member calendars in viewer timezone.
- Add DST policy tests.

Acceptance: different viewer timezones see equivalent actual instants at corresponding local times; week boundaries are correct; API and UI use same conversion data.

## Phase 7: Best-time and PNG
- Implement preference ranking using converted reference-week slots.
- Merge adjacent equal-score slots for presentation.
- Generate SVG and convert to PNG.
- Ensure image uses same timetable view model as interactive UI.

Acceptance: pending members excluded; completed-empty members handled correctly; PNG includes required labels and matches API result.

## Phase 8: Polish and deployment
- Complete Russian copy and empty/loading/error states.
- Add group cap and permission UX.
- Configure HTTPS deployment and persistent project-local data volume.
- Document that automated backups are not included and data loss may be permanent.
- Test restart and migration behavior without replacing database.

Acceptance: clean deployment instructions work; restart preserves DB; secrets/data are not committed; Mini App opens in Telegram over HTTPS.

## Testing priorities
1. Timezone conversion, upcoming Monday logic and DST boundaries.
2. Invitation replay, expiry, revocation and simultaneous joins at capacity.
3. Ownership transfer and owner-leave restrictions.
4. Preference replacement, empty save, overlap and interval bounds.
5. Group-scoped preference cascade.
6. Mini App authentication and authorization.
7. PNG/interactive timetable consistency.
8. Notification failure does not roll back writes.
