use sqlx::{Connection, Executor, Sqlite, SqlitePool};

use crate::{
    db,
    domain::{
        GroupId, PreferenceError, PreferenceInterval, SetupState, UserId, Weekday,
        normalize_intervals,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberPreferences {
    pub group_id: GroupId,
    pub intervals: Vec<PreferenceInterval>,
    pub setup_completed_at: Option<i64>,
}

impl MemberPreferences {
    pub fn setup_state(&self) -> SetupState {
        SetupState::from_completed_at(self.setup_completed_at)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedPreferences {
    pub group_id: GroupId,
    pub intervals: Vec<PreferenceInterval>,
    pub setup_completed_at: Option<i64>,
    /// True when the normalized interval list differs from the previous one.
    pub changed: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum PreferencesError {
    #[error("group {0} is not available to this user")]
    NotAMember(GroupId),
    #[error(transparent)]
    Validation(#[from] PreferenceError),
    #[error("stored preference data is invalid: {0}")]
    CorruptData(String),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

/// Loads the authenticated member's local recurring intervals.
pub async fn get_own(
    pool: &SqlitePool,
    user: UserId,
    group: GroupId,
) -> Result<MemberPreferences, PreferencesError> {
    let setup_completed_at: Option<Option<i64>> = sqlx::query_scalar(
        "SELECT setup_completed_at FROM group_members WHERE group_id = ? AND user_id = ?",
    )
    .bind(group.get())
    .bind(user.get())
    .fetch_optional(pool)
    .await?;
    let Some(setup_completed_at) = setup_completed_at else {
        return Err(PreferencesError::NotAMember(group));
    };

    let intervals = load_intervals(pool, group, user).await?;
    Ok(MemberPreferences {
        group_id: group,
        intervals,
        setup_completed_at,
    })
}

/// Replaces all intervals for one member inside a single write transaction.
///
/// An empty list is valid and marks setup completed. `setup_completed_at`
/// records the first successful save and is never cleared.
pub async fn replace(
    pool: &SqlitePool,
    user: UserId,
    group: GroupId,
    intervals: Vec<PreferenceInterval>,
) -> Result<SavedPreferences, PreferencesError> {
    let normalized = normalize_intervals(intervals)?;

    let mut connection = pool.acquire().await?;
    let mut transaction = connection.begin_with(db::BEGIN_IMMEDIATE).await?;

    let existing: Option<Option<i64>> = sqlx::query_scalar(
        "SELECT setup_completed_at FROM group_members WHERE group_id = ? AND user_id = ?",
    )
    .bind(group.get())
    .bind(user.get())
    .fetch_optional(&mut *transaction)
    .await?;
    let Some(existing_completed_at) = existing else {
        return Err(PreferencesError::NotAMember(group));
    };

    let previous = load_intervals(&mut *transaction, group, user).await?;
    let changed = previous != normalized;

    sqlx::query("DELETE FROM weekly_preferences WHERE group_id = ? AND user_id = ?")
        .bind(group.get())
        .bind(user.get())
        .execute(&mut *transaction)
        .await?;

    for interval in &normalized {
        sqlx::query(
            "INSERT INTO weekly_preferences
                 (group_id, user_id, weekday, start_minute, end_minute)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(group.get())
        .bind(user.get())
        .bind(i64::from(interval.weekday().number()))
        .bind(i64::from(interval.start_minute()))
        .bind(i64::from(interval.end_minute()))
        .execute(&mut *transaction)
        .await?;
    }

    let now = super::now_millis();
    let setup_completed_at = existing_completed_at.unwrap_or(now);
    sqlx::query(
        "UPDATE group_members
         SET setup_completed_at = COALESCE(setup_completed_at, ?)
         WHERE group_id = ? AND user_id = ?",
    )
    .bind(now)
    .bind(group.get())
    .bind(user.get())
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;

    Ok(SavedPreferences {
        group_id: group,
        intervals: normalized,
        setup_completed_at: Some(setup_completed_at),
        changed,
    })
}

async fn load_intervals<'e, E>(
    executor: E,
    group: GroupId,
    user: UserId,
) -> Result<Vec<PreferenceInterval>, PreferencesError>
where
    E: Executor<'e, Database = Sqlite>,
{
    let rows: Vec<(i64, i64, i64)> = sqlx::query_as(
        "SELECT weekday, start_minute, end_minute
         FROM weekly_preferences
         WHERE group_id = ? AND user_id = ?
         ORDER BY weekday, start_minute",
    )
    .bind(group.get())
    .bind(user.get())
    .fetch_all(executor)
    .await?;

    let mut intervals = Vec::with_capacity(rows.len());
    for (weekday, start, end) in rows {
        let weekday = Weekday::from_db(weekday)
            .map_err(|error| PreferencesError::CorruptData(error.to_string()))?;
        let start = u16::try_from(start)
            .map_err(|_| PreferencesError::CorruptData("start_minute out of range".to_owned()))?;
        let end = u16::try_from(end)
            .map_err(|_| PreferencesError::CorruptData("end_minute out of range".to_owned()))?;
        intervals.push(
            PreferenceInterval::new(weekday, start, end)
                .map_err(|error| PreferencesError::CorruptData(error.to_string()))?,
        );
    }

    normalize_intervals(intervals).map_err(|error| PreferencesError::CorruptData(error.to_string()))
}
