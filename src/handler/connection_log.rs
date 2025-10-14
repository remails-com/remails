use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct ConnectionLog {
    lines: Vec<LogLine>,
}

#[derive(Debug, Deserialize, Serialize)]
struct LogLine {
    time: DateTime<Utc>,
    level: LogLevel,
    msg: String,
}

impl ConnectionLog {
    pub fn log(&mut self, level: LogLevel, msg: impl Display) {
        let line = LogLine {
            time: Utc::now(),
            level,
            msg: msg.to_string(),
        };
        self.lines.push(line);
    }
}
