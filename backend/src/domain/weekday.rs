/// Weekday in the recurring weekly calendar. Monday is 1, Sunday is 7.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum Weekday {
    Monday = 1,
    Tuesday = 2,
    Wednesday = 3,
    Thursday = 4,
    Friday = 5,
    Saturday = 6,
    Sunday = 7,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("weekday must be in 1..=7, got {0}")]
pub struct WeekdayError(pub i64);

impl Weekday {
    pub const ALL: [Weekday; 7] = [
        Weekday::Monday,
        Weekday::Tuesday,
        Weekday::Wednesday,
        Weekday::Thursday,
        Weekday::Friday,
        Weekday::Saturday,
        Weekday::Sunday,
    ];

    pub fn number(self) -> u8 {
        self as u8
    }

    pub fn from_number(value: u8) -> Result<Self, WeekdayError> {
        Self::from_db(i64::from(value))
    }

    pub fn from_db(value: i64) -> Result<Self, WeekdayError> {
        match value {
            1 => Ok(Weekday::Monday),
            2 => Ok(Weekday::Tuesday),
            3 => Ok(Weekday::Wednesday),
            4 => Ok(Weekday::Thursday),
            5 => Ok(Weekday::Friday),
            6 => Ok(Weekday::Saturday),
            7 => Ok(Weekday::Sunday),
            other => Err(WeekdayError(other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_weekdays_round_trip() {
        for weekday in Weekday::ALL {
            assert_eq!(
                Weekday::from_db(i64::from(weekday.number())).unwrap(),
                weekday
            );
        }
    }

    #[test]
    fn monday_is_first_and_sunday_is_last() {
        assert_eq!(Weekday::Monday.number(), 1);
        assert_eq!(Weekday::Sunday.number(), 7);
        assert!(Weekday::Monday < Weekday::Sunday);
    }

    #[test]
    fn rejects_out_of_range_values() {
        for value in [-1, 0, 8, 100] {
            assert!(Weekday::from_db(value).is_err(), "{value} must be rejected");
        }
    }
}
