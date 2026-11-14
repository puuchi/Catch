use chrono::{Days, Local, NaiveDate, TimeZone};
pub fn now() -> i64 {
    chrono::Utc::now().timestamp_millis()
}
pub fn day(time: i64) -> String {
    Local
        .timestamp_millis_opt(time)
        .single()
        .unwrap()
        .format("%Y-%m-%d")
        .to_string()
}
pub fn bounds(date: &str) -> Result<(i64, i64), String> {
    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| "日期无效")?;
    let tomorrow = date.checked_add_days(Days::new(1)).ok_or("日期超出范围")?;
    fn start(date: NaiveDate) -> Result<i64, String> {
        // A timezone transition can skip midnight. Choose the first valid
        // minute in that local date; ambiguous midnight uses its earliest side.
        for minute in 0..1440 {
            let time = date.and_hms_opt(minute / 60, minute % 60, 0).unwrap();
            if let Some(value) = Local.from_local_datetime(&time).earliest() {
                return Ok(value.timestamp_millis());
            }
        }
        Err("当地时区中不存在这一天".into())
    }
    Ok((start(date)?, start(tomorrow)?))
}
