use re_log_types::{Time, TimePoint, TimeType, Timeline};

pub fn log_time() -> TimePoint {
    TimePoint(
        [(
            Timeline::new("log_time", TimeType::Time),
            Time::now().into(),
        )]
        .into(),
    )
}
