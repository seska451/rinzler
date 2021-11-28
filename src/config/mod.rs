use bitflags::bitflags;
use std::fmt::{Display, Formatter};
use tracing::Level;

bitflags! {
    pub struct Flags: u8 {
        const NONE = 0x0;
        const SCOPED = 0x01;
        const UNSCOPED = 0x02;
        const FUZZ = 0x16;
        const BRUTE = 0x32;
        const CRAWL = 0x64;
    }
}

pub struct Settings {
    pub user_agent: String,
    pub rate_limit: u64,
    pub scoped: bool,
    pub verbosity: Level,
    pub quiet: bool,
    pub hosts: Vec<String>,
    pub recurse: bool,
    pub wordlist: Option<Vec<String>>,
    pub wordlist_filename: Option<String>,
    pub status_include: Vec<u16>,
    pub status_exclude: Vec<u16>,
    pub flags: Flags,
    pub max_threads: usize,
}

impl Clone for Settings {
    fn clone(&self) -> Self {
        Settings {
            user_agent: self.user_agent.clone(),
            rate_limit: self.rate_limit.clone(),
            scoped: self.scoped,
            verbosity: self.verbosity,
            quiet: self.quiet,
            hosts: self.hosts.clone(),
            recurse: self.recurse,
            wordlist: self.wordlist.clone(),
            wordlist_filename: self.wordlist_filename.clone(),
            status_include: self.status_include.clone(),
            status_exclude: self.status_exclude.clone(),
            flags: self.flags.clone(),
            max_threads: 50,
        }
    }
}

impl Display for Settings {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "  Flags:       {:?}", self.flags)?;
        if !self.status_include.is_empty() {
            let status_inc: Vec<String> =
                self.status_include.iter().map(|n| n.to_string()).collect();
            writeln!(f, "  Included status:  {}", status_inc.join(", "))?;
        }
        if !self.status_exclude.is_empty() {
            let status_ex: Vec<String> =
                self.status_exclude.iter().map(|n| n.to_string()).collect();
            writeln!(f, "  Excluded status:  {}", status_ex.join(", "))?;
        }
        writeln!(f, "  User-Agent:  {}", self.user_agent)?;
        writeln!(f, "  Throttle:    {}ms", self.rate_limit)?;
        writeln!(f, "  Log Level:   {}", self.verbosity)?;
        writeln!(f, "  Targets:     {}", self.hosts.join(", "))?;
        Ok(match &self.wordlist_filename {
            Some(wl) => writeln!(
                f,
                "  Wordlist {} with {} words",
                wl,
                match &self.wordlist {
                    Some(w) => w.len(),
                    None => 0,
                }
            )?,
            None => write!(f, "")?,
        })
    }
}
