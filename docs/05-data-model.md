# 05. Data Model (SQLite)

This is a logical schema proposal. The implementation agent may refine names/types, but must preserve the invariants.

## 1. `users`
- `id`: internal integer primary key.
- `telegram_id`: unique Telegram numeric identifier. Store safely as SQLite INTEGER (signed 64-bit) if validated to fit; expose as string in JSON/JavaScript to avoid precision loss.
- `username`: nullable, mutable.
- `first_name`: nullable/current display name.
- `last_name`: nullable/current display name.
- `timezone`: nullable IANA identifier until selected.
- `created_at`, `updated_at`.

Do not use username or display name as identity keys.

## 2. `groups`
- `id`: primary key (UUID/text or integer; choose one consistently).
- `name`: non-empty trimmed name.
- `owner_user_id`: FK to users.
- `created_at`, `updated_at`.
- Enforce owner membership at service/transaction level. SQLite cannot express every cross-table invariant as a simple FK; add tests and transactional checks.

## 3. `group_members`
- `group_id`: FK groups with `ON DELETE CASCADE`.
- `user_id`: FK users with `ON DELETE CASCADE`.
- `role`: `owner` or `member` (owner role should correspond to groups.owner_user_id).
- `color`: stable color value assigned within group.
- `setup_completed_at`: nullable; NULL means pending, non-NULL means setup completed (even if zero preferences).
- `joined_at`.
- Composite primary key / unique constraint `(group_id, user_id)`.
- Enforce maximum 20 members transactionally. Count and insert under a write transaction; handle SQLite busy/constraint outcomes.

## 4. `weekly_preferences`
- `id`: primary key.
- `group_id`, `user_id`: composite membership reference where practical; ensure preferences cannot exist for a non-member.
- `weekday`: Monday=1 ... Sunday=7.
- `start_minute`: integer 0..1439, multiple of 30.
- `end_minute`: integer 1..1440, multiple of 30, greater than start.
- Unique/overlap validation: no overlapping intervals for same group/user/weekday. Adjacent intervals are allowed. SQLite CHECK constraints can validate bounds and multiples, but overlap should be checked transactionally in service logic (or trigger if implementation chooses).
- Index `(group_id, user_id, weekday, start_minute)`.

Preference save replaces all rows for `(group_id,user_id)` inside one transaction and updates `setup_completed_at`. Empty input is valid. Compare normalized old/new intervals before writing to determine whether to notify owner about a changed preference save.

## 5. `invitations`
- `id`: primary key.
- `group_id`: FK groups with cascade.
- `token_hash`: unique; never store raw token.
- `created_by_user_id`: FK users.
- `created_at`, nullable `expires_at`, nullable `revoked_at`.
- Optional label/metadata only if needed; avoid overbuilding.

## 6. `notification_log` (optional)
Can be omitted initially if structured application logs are sufficient. If implemented, store event type, recipient, status, error category and timestamp. Never store bot token or raw auth/invitation secrets. Notification failures must not affect domain transactions.

## 7. Referential integrity
- Enable `PRAGMA foreign_keys=ON` on every connection.
- Group deletion cascades membership, preferences and invitations.
- Membership deletion cascades only that member's preferences for that group.
- User profile remains if they leave a group.
- Leaving group A must not affect group B preferences.

## 8. Constraints and transaction rules
- Unique `(group_id,user_id)` membership.
- Maximum 20 members per group.
- One owner per group, reflected consistently in `groups.owner_user_id` and membership role.
- Invitation join: validate + count + insert atomically.
- Ownership transfer: validate owner and target membership + update owner and roles atomically.
- Preference save: validate normalized intervals + replace rows + update setup state atomically.
- Group deletion: owner authorization and deletion transaction; capture recipients before deletion for best-effort notifications.

## 9. Git ignore
At minimum:
```gitignore
/data/
.env
*.db
*.db-wal
*.db-shm
```
Database path is project-local `data/app.db`. No automated backups in MVP.
