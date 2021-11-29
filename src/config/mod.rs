use bitflags::bitflags;
use clap::{App, Arg, ArgMatches};
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{BufRead, BufReader};
use tracing::{debug, error, info, trace, Level};

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

pub struct RinzlerSettings {
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

impl Clone for RinzlerSettings {
    fn clone(&self) -> Self {
        RinzlerSettings {
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
            max_threads: self.max_threads.clone(),
        }
    }
}

impl Display for RinzlerSettings {
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
        writeln!(f, "  Threads:     {}", self.max_threads)?;
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

pub(crate) fn parse_cmd_line() -> RinzlerSettings {
    let args = App::new("rinzler")
        .version(env!("CARGO_PKG_VERSION"))
        .author("seska <seska@seska.io>")
        .about("A simple to use, multithreading web crawler, fuzzer and vulnerability scanner.")
        .arg(Arg::new("single_host")
            .index(1)
            .conflicts_with("host")
            .required(true)
            .value_name("HOST URL")
            .about("The host URL to scan"))
        .arg(Arg::new("shallow")
            .short('S')
            .long("shallow")
            .conflicts_with("deep")
            .takes_value(false)
            .about("Indicates use of a shallow (non-recursive) scan. By default a deep crawl (recursive) is performed, unless fuzzing or forced browsing is used."))
        .arg(Arg::new("deep")
            .short('D')
            .long("deep")
            .conflicts_with("shallow")
            .takes_value(false)
            .about("Indicates use of a deep (recursive) scan. This is done by default, unless fuzzing or forced browsing is used."))
        .arg(Arg::new("host")
            .short('h')
            .long("host")
            .value_name("HOST URL")
            .multiple_occurrences(true)
            .env("RINZLER_HOSTS")
            .takes_value(true)
            .about("Set the initial URL to start crawling. Can be set multiple times to crawl several sites at once."))
        .arg(Arg::new("verbosity")
            .short('v')
            .multiple_occurrences(true)
            .about("Sets the level of output verbosity. Set multiple times "))
        .arg(Arg::new("quiet")
            .short('q')
            .long("quiet")
            .takes_value(false)
            .default_value("false")
            .about("When set, this flag suppresses extraneous output like the version banner."))
        .arg(Arg::new("scoped")
            .short('s')
            .long("scoped")
            .default_value("true")
            .about("Prevents rinzler from searching beyond the original domains specified. Defaults to true."))
        .arg(Arg::new("user-agent")
            .short('u')
            .long("user-agent")
            .env("RINZLER_UA")
            .takes_value(true)
            .default_value(format!("rinzler v{}", env!("CARGO_PKG_VERSION")).as_str())
            .about(format!("Set the user-agent header. Defaults to '{}'", env!("CARGO_PKG_VERSION")).as_str()))
        .arg(Arg::new("rate-limit")
            .short('r')
            .long("rate-limit")
            .env("RINZLER_RATE_LIMIT")
            .takes_value(true)
            .default_value("0")
            .about("Set the number of milliseconds to wait between each request."))
        .arg(Arg::new("wordlist")
            .short('w')
            .long("wordlist")
            .takes_value(true)
            .env("RINZLER_WORDLIST")
            .about("Supply a wordlist to perform forced browsing"))
        .arg(Arg::new("status-include")
            .short('i')
            .long("status-include")
            .takes_value(true)
            .min_values(1)
            .about("Set the status codes you're interested in."))
        .arg(Arg::new("status-exclude")
            .short('e')
            .long("status-exclude")
            .takes_value(true)
            .min_values(1)
            .about("Set the status codes you're not interested in."))
        .arg(Arg::new("threads")
            .short('t')
            .long("threads")
            .takes_value(true)
            .required(false)
            .env("RINZLER_THREADS")
            .default_value("50")
            .about("Set the maximum number of threads to build the thread pool that rinzler uses when processing targets."))
        .get_matches().to_owned();

    let mut settings = RinzlerSettings {
        user_agent: match args.value_of("user-agent") {
            Some(ua) => ua.to_string(),
            None => env!("CARGO_PKG_VERSION").to_string(),
        },
        rate_limit: args.value_of("rate-limit").unwrap().parse::<u64>().unwrap(),
        scoped: args.value_of("scoped").unwrap().parse::<bool>().unwrap(),
        recurse: match args.is_present("wordlist") {
            true => args.is_present("deep"),
            false => !args.is_present("shallow"),
        },
        wordlist_filename: match args.value_of("wordlist") {
            Some(wl) => Some(wl.to_string()),
            None => None,
        },
        wordlist: match args.value_of("wordlist") {
            Some(wl) => {
                debug!("Loading wordlist from {}", wl);
                let file = File::open(wl).unwrap();
                let reader = BufReader::new(file);
                let mut urls = Vec::new();
                for line in reader.lines() {
                    if !line.as_ref().unwrap().starts_with('#') {
                        urls.push(line.unwrap().to_string())
                    }
                }
                Some(urls)
            }
            None => None,
        },
        status_include: match args.values_of_t::<u16>("status-include") {
            Ok(v) => v,
            Err(_) => vec![],
        },
        status_exclude: match args.values_of_t::<u16>("status-exclude") {
            Ok(v) => v,
            Err(_) => vec![],
        },
        verbosity: match args.occurrences_of("verbosity") {
            0 => Level::WARN,
            1 => Level::INFO,
            2 => Level::DEBUG,
            _ => Level::TRACE,
        },
        quiet: args.value_of_t::<bool>("quiet").unwrap(),
        max_threads: {
            let t = args.value_of_t::<usize>("threads").unwrap();
            if t > 0 && t <= 1000 {
                t
            } else {
                if t <= 0 {
                    error!("Need at least one thread to work with, plzkthxbai!");
                }
                if t > 1000 {
                    error!("Things get weird over 1000 threads!");
                }
                50
            }
        },
        hosts: get_hosts_from_args(args),
        flags: Flags::NONE,
    };

    pre_configure(&mut settings);
    configure_logging(settings.verbosity);
    settings
}

fn get_hosts_from_args(args: ArgMatches) -> Vec<String> {
    match args.values_of_lossy("host") {
        Some(hosts) => hosts,
        None => {
            let single_host = args.value_of("single_host").unwrap().to_string();
            let mut vec: Vec<String> = Vec::new();
            vec.push(single_host);
            vec
        }
    }
}

fn pre_configure(settings: &mut RinzlerSettings) {
    settings.flags = if settings.scoped {
        Flags::SCOPED
    } else {
        Flags::UNSCOPED
    };
    settings.flags |= if settings.recurse {
        Flags::CRAWL
    } else {
        if settings.hosts.iter().any(|h| h.contains("FUZZ")) {
            Flags::FUZZ
        } else {
            Flags::BRUTE
        }
    };

    exclude_not_found_if_force_browsing(settings);
}

fn exclude_not_found_if_force_browsing(settings: &mut RinzlerSettings) {
    if !settings.recurse && settings.status_exclude.is_empty() {
        settings.status_exclude = vec![404];
    }
}

fn configure_logging(verbosity_level: Level) {
    tracing_subscriber::fmt()
        .with_max_level(verbosity_level)
        .init();
    info!("Verbosity level set to {}", verbosity_level);
    trace!("configured logging");
}
