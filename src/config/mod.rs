use std::fmt::{Display, Formatter};
use console::Emoji;
use tracing::Level;

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
}

static SWISS_FLAG: Emoji = Emoji("  🇨🇭  ", ":");
static DETECTIVE: Emoji = Emoji("  🕵🏼  ", ":");
static RACING_CAR: Emoji = Emoji("  🏎️  ", ":");
static NERD_FACE: Emoji = Emoji("  🤓  ", ":");
static GLOBE: Emoji = Emoji("  🌐  ", ":");
static GREEN_CHECK: Emoji = Emoji("  ✅  ", ":");
static CROSS_MARK: Emoji = Emoji("  ❌  ", ":");
static MECHANICAL_ARM: Emoji = Emoji("  🦾  ", ":");

impl Display for Settings {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "")?;
        writeln!(f, "      FLAGS{} {} {} {}", SWISS_FLAG, match self.scoped {
            true => "SCOPED",
            false => "UNSCOPED",
        }, match self.recurse {
            true => "DEEP",
            false => "SHALLOW"
        }, match self.wordlist_filename.is_some() {
            true => "BRUTE",
            false => ""
        })?;
        if !self.status_include.is_empty() {
            let status_inc: Vec<String> = self.status_include.iter().map(|n| n.to_string()).collect();
            writeln!(f, "StatusCodes{}{}", GREEN_CHECK, status_inc.join(", "))?;
        }
        if !self.status_exclude.is_empty() {
            let status_ex: Vec<String> = self.status_exclude.iter().map(|n| n.to_string()).collect();
            writeln!(f, "StatusCodes{}{}", CROSS_MARK, status_ex.join(", "))?;
        }
        writeln!(f, " User agent{}{}", DETECTIVE, self.user_agent)?;
        writeln!(f, " Rate limit{}{}ms", RACING_CAR, self.rate_limit)?;
        writeln!(f, "  Verbosity{}{}", NERD_FACE, self.verbosity)?;
        writeln!(f, "      Hosts{}{}", GLOBE, self.hosts.join(", "))?;
        match &self.wordlist_filename {
            Some(wl) => {
                writeln!(f, "   Wordlist{}{} with {} words", MECHANICAL_ARM, wl, match &self.wordlist {
                    Some(w) => w.len(),
                    None => 0
                })?
            },
            None => write!(f, "")?
        };
        writeln!(f, "=============================================\n")
    }
}