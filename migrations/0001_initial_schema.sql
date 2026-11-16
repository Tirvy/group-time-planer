-- 0001_initial_schema.sql
--
-- Timestamps are UTC milliseconds since the Unix epoch (INTEGER). This keeps
-- comparisons unambiguous and avoids SQLite text-date parsing differences.
--
-- Invariants implemented here:
--   * Telegram IDs are unique and positive.
--   * Group names are trimmed and 1..=64 characters.
--   * Membership is unique per (group, user) and capped at 20 including owner.
--   * At most one owner membership per group.
--   * Preferences belong to an existing membership and cannot overlap.
--   * Group deletion cascades memberships, preferences and invitations.
--   * Membership deletion cascades only that member's preferences.

CREATE TABLE users (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    telegram_id  INTEGER NOT NULL UNIQUE CHECK (telegram_id > 0),
    username     TEXT,
    first_name   TEXT,
    last_name    TEXT,
    timezone     TEXT,
    created_at   INTEGER NOT NULL,
    updated_at   INTEGER NOT NULL
);

CREATE TABLE groups (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    name           TEXT NOT NULL
                   CHECK (length(trim(name)) BETWEEN 1 AND 64),
    owner_user_id  INTEGER NOT NULL REFERENCES users (id) ON DELETE RESTRICT,
    created_at     INTEGER NOT NULL,
    updated_at     INTEGER NOT NULL
);

CREATE INDEX idx_groups_owner ON groups (owner_user_id);

CREATE TABLE group_members (
    group_id            INTEGER NOT NULL REFERENCES groups (id) ON DELETE CASCADE,
    user_id             INTEGER NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    role                TEXT NOT NULL CHECK (role IN ('owner', 'member')),
    color               TEXT NOT NULL
                        CHECK (color GLOB '#[0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f]'),
    setup_completed_at  INTEGER,
    joined_at           INTEGER NOT NULL,
    PRIMARY KEY (group_id, user_id)
);

-- Only one owner membership per group.
CREATE UNIQUE INDEX idx_group_members_single_owner
    ON group_members (group_id)
    WHERE role = 'owner';

-- Listing "my groups" is a frequent query.
CREATE INDEX idx_group_members_user ON group_members (user_id);

-- Defense in depth: the service checks capacity transactionally and returns a
-- stable error code, but this trigger guarantees the cap even for direct writes.
CREATE TRIGGER trg_group_members_capacity
BEFORE INSERT ON group_members
FOR EACH ROW
WHEN (SELECT COUNT(*) FROM group_members WHERE group_id = NEW.group_id) >= 20
BEGIN
    SELECT RAISE(ABORT, 'group_full');
END;

CREATE TABLE weekly_preferences (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    group_id      INTEGER NOT NULL,
    user_id       INTEGER NOT NULL,
    weekday       INTEGER NOT NULL CHECK (weekday BETWEEN 1 AND 7),
    start_minute  INTEGER NOT NULL
                  CHECK (start_minute >= 0 AND start_minute <= 1439 AND start_minute % 30 = 0),
    end_minute    INTEGER NOT NULL
                  CHECK (end_minute >= 1 AND end_minute <= 1440 AND end_minute % 30 = 0),
    CHECK (start_minute < end_minute),
    FOREIGN KEY (group_id, user_id)
        REFERENCES group_members (group_id, user_id) ON DELETE CASCADE
);

CREATE INDEX idx_weekly_preferences_member
    ON weekly_preferences (group_id, user_id, weekday, start_minute);

-- Intervals are half-open [start, end); adjacency is allowed, overlap is not.
CREATE TRIGGER trg_weekly_preferences_no_overlap_insert
BEFORE INSERT ON weekly_preferences
FOR EACH ROW
WHEN EXISTS (
    SELECT 1
    FROM weekly_preferences p
    WHERE p.group_id = NEW.group_id
      AND p.user_id = NEW.user_id
      AND p.weekday = NEW.weekday
      AND NEW.start_minute < p.end_minute
      AND p.start_minute < NEW.end_minute
)
BEGIN
    SELECT RAISE(ABORT, 'overlapping_intervals');
END;

CREATE TRIGGER trg_weekly_preferences_no_overlap_update
BEFORE UPDATE OF group_id, user_id, weekday, start_minute, end_minute
ON weekly_preferences
FOR EACH ROW
WHEN EXISTS (
    SELECT 1
    FROM weekly_preferences p
    WHERE p.id <> OLD.id
      AND p.group_id = NEW.group_id
      AND p.user_id = NEW.user_id
      AND p.weekday = NEW.weekday
      AND NEW.start_minute < p.end_minute
      AND p.start_minute < NEW.end_minute
)
BEGIN
    SELECT RAISE(ABORT, 'overlapping_intervals');
END;

CREATE TABLE invitations (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    group_id            INTEGER NOT NULL REFERENCES groups (id) ON DELETE CASCADE,
    token_hash          TEXT NOT NULL UNIQUE,
    created_by_user_id  INTEGER NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at          INTEGER NOT NULL,
    expires_at          INTEGER,
    revoked_at          INTEGER,
    CHECK (expires_at IS NULL OR expires_at > created_at),
    CHECK (revoked_at IS NULL OR revoked_at >= created_at)
);

CREATE INDEX idx_invitations_group ON invitations (group_id);
