use chrono::{DateTime, Local};
use colored::{ColoredString, Colorize};
use reqwest::blocking::Response;
use reqwest::Url;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use uuid::Uuid;

pub struct CrawlTarget {
    pub(crate) id: Uuid,
    pub status_code: Option<u16>,
    pub url: String,
    pub(crate) timestamp: DateTime<Local>,
}

impl CrawlTarget {
    pub fn from_url(u: Url) -> CrawlTarget {
        CrawlTarget {
            id: Uuid::new_v4(),
            status_code: None,
            url: u.to_string(),
            timestamp: Local::now(),
        }
    }
}

impl Display for CrawlTarget {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(status_code) = self.status_code {
            let fmt_status = Self::fmt_status_code(status_code);
            write!(
                f,
                "{} {} {}",
                self.timestamp.format("%T%.3f%z"),
                fmt_status,
                self.url.as_str().cyan()
            )
        } else {
            write!(
                f,
                "{} ??? {}",
                self.timestamp.format("%T%.3f%z"),
                self.url.as_str().cyan()
            )
        }
    }
}

impl PartialEq<Self> for CrawlTarget {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for CrawlTarget {}

impl Hash for CrawlTarget {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(self.id.as_bytes());
        state.finish();
    }
}

impl Clone for CrawlTarget {
    fn clone(&self) -> Self {
        CrawlTarget {
            id: self.id.clone(),
            status_code: self.status_code.clone(),
            url: self.url.clone(),
            timestamp: self.timestamp.clone(),
        }
    }
}

impl CrawlTarget {
    pub fn new() -> CrawlTarget {
        CrawlTarget {
            id: Uuid::new_v4(),
            status_code: None,
            url: String::default(),
            timestamp: Local::now(),
        }
    }

    pub fn from_response(res: Response) -> CrawlTarget {
        CrawlTarget {
            id: Uuid::new_v4(),
            status_code: Some(res.status().as_u16()),
            url: res.url().to_string(),
            timestamp: Local::now(),
        }
    }

    fn fmt_status_code(status_code: u16) -> ColoredString {
        match status_code {
            0..=199 => status_code.to_string().as_str().bright_white(),
            200..=299 => status_code.to_string().as_str().green(),
            300..=399 => status_code.to_string().as_str().bright_yellow(),
            400..=499 => status_code.to_string().as_str().yellow(),
            _ => status_code.to_string().as_str().red(),
        }
    }
}
