//! Mirrors `app_crates/domain_google/src/timezone_conversion.rs`.
//! Only load-bearing conversion is ported. Calendar call sites do not use
//! the other conversion helpers from the source module.

use time::{Date, PrimitiveDateTime, Time};
use time_tz::{OffsetDateTimeExt, PrimitiveDateTimeExt, Tz, timezones};

pub const REFERENCE_TIMEZONE: &str = "Europe/Paris";

/// Availability rules - times in reference timezone (Paris UTC+1/+2)
pub const SLOT_TIMES_REFERENCE: [&str; 12] = [
    "13:00", "13:30", "14:00", "14:30", "15:00", "15:30", "16:00", "16:30", "17:00", "17:30",
    "18:00", "18:30",
];

/// Result of a time conversion that may cross day boundaries
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConvertedTime {
    pub time: String,
    pub day_offset: i8, // -1 = previous day, 0 = same day, 1 = next day
}

fn get_tz(iana: &str) -> Option<&'static Tz> {
    timezones::get_by_name(iana)
}

pub fn convert_time_between_tz(
    time_str: &str,
    date: Date,
    from_tz: &str,
    to_tz: &str,
) -> Option<ConvertedTime> {
    let from_timezone = get_tz(from_tz)?;
    let to_timezone = get_tz(to_tz)?;

    let (hour_str, min_str) = time_str.split_once(':')?;
    let hour: u8 = hour_str.parse().ok()?;
    let min: u8 = min_str.parse().ok()?;
    let time = Time::from_hms(hour, min, 0).ok()?;

    let datetime = PrimitiveDateTime::new(date, time);
    let from_dt = datetime.assume_timezone(from_timezone).unwrap();
    let to_dt = from_dt.to_timezone(to_timezone);

    let day_offset = i8::try_from((to_dt.date() - date).whole_days()).unwrap_or(0);

    Some(ConvertedTime {
        time: format!("{:02}:{:02}", to_dt.hour(), to_dt.minute()),
        day_offset,
    })
}
