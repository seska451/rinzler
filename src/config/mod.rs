use std::vec::IntoIter;
use tracing::Level;

pub struct Settings {
    pub user_agent: String,
    pub rate_limit: u64,
    pub scoped: bool,
    pub verbosity: Level,
    pub quiet: bool,
    pub hosts: IntoIter<String>,
    pub recurse: bool,
    pub wordlist: Option<String>,
}