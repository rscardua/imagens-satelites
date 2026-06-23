use chrono::{DateTime, Utc};

use crate::application::ports::Clock;

/// Relógio de sistema (UTC).
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}
