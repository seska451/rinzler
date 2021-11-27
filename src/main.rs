use std::{env, thread};
use std::fs::File;
use std::io::{prelude::*, BufReader};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use clap::{Arg, App, ArgMatches};
use crossbeam::channel::{unbounded, Receiver, Sender};
use threadpool::ThreadPool;
use tracing::{info, trace, Level, debug};
use tracing_subscriber;
use url::Url;
use config::Settings;
use ui::rinzler_console::RinzlerConsole;
pub use crate::{
    config::Flags,
    crawler::rinzler_crawler::RinzlerCrawler,
    crawler::crawl_target::CrawlTarget,
    crawler::rinzler_crawler::{ControllerMessage, ControllerMessageType}
};
use crate::ui::rinzler_console::{ConsoleMessage, ConsoleMessageType};

mod crawler;
mod config;
mod ui;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = parse_cmd_line();
    configure_logging(settings.verbosity);
    let (console_sender, message_receiver) = unbounded();
    let console = RinzlerConsole::new(settings.clone(), message_receiver)?;
    let thread_pool = threadpool::ThreadPool::new(settings.max_threads);
    let mut controller_receivers = vec![];
    let visited = Arc::new(Mutex::new(vec![]));
    let scoped_domains : Vec<String> = settings.hosts.iter().map(|h| Url::parse(h).unwrap().domain().unwrap().to_string()).collect();
    start_console(console, &thread_pool, settings.clone());
    start_crawlers(settings.clone(), console_sender.clone(), &thread_pool, settings.hosts.clone(), &mut controller_receivers, visited, scoped_domains.clone());
    let outcome = wait_for_crawlers_to_finish(&mut controller_receivers);

    inform_console_to_exit(outcome, console_sender.clone());

    thread_pool.join();

    Ok(())
}

fn inform_console_to_exit(reason: Result<String, String>, command_tx: Sender<ConsoleMessage>) {
    thread::sleep(Duration::from_secs(1));
    let _ = command_tx.send(ConsoleMessage {
        message_type: ConsoleMessageType::FINISH,
        data: reason,
        crawl_target: None
    });
}

fn wait_for_crawlers_to_finish(controller_receivers: &mut Vec<Receiver<ControllerMessage>>) -> Result<String, String> {
    let mut errors = vec![];
    loop {
        let finished = controller_receivers.iter_mut().all(|r| {
            if let Ok(fin) = r.recv() {
                match fin.message_type {
                    ControllerMessageType::FINISHED => true,
                    ControllerMessageType::ERROR => {
                        errors.push(fin.data);
                        true
                    }
                }
            } else {
                false
            }
        });
        thread::sleep(Duration::from_millis(100));
        if !errors.is_empty() {
            break;
        }
        if finished {
            break;
        }
    }

    if errors.is_empty() {
        Ok("Scan Completed".to_string())
    } else {
        let _ =format!("{}", errors.to_owned().join("\n")).as_str();
        Err("Scan Failed".to_string())
    }
}

fn start_crawlers(settings: Settings, console_sender: Sender<ConsoleMessage>, thread_pool: &ThreadPool, hosts: Vec<String>, controller_receivers: &mut Vec<Receiver<ControllerMessage>>, visited:Arc<Mutex<Vec<String>>>, scoped_domains: Vec<String>) {
    for target in hosts {
        let settings = settings.clone();
        let (controller_sender, controller_receiver) = unbounded();
        let console_sender = console_sender.clone();
        let v = Arc::clone(&visited);
        let scoped_domains = scoped_domains.clone();
        thread_pool.execute(move || {
            let crawler = RinzlerCrawler::new(target, settings, controller_sender, console_sender, scoped_domains);
            let result = crawler.crawl(v);
            if let Ok(_result) = result {
                crawler.finish()
            }
        });
        controller_receivers.push(controller_receiver);
    }
}

fn start_console(console: RinzlerConsole, thread_pool: &ThreadPool, settings_: Settings) {
    thread_pool.execute(move || {
        console
            .clear()
            .banner(format!("{}", settings_.clone()))
            .render();
    });
}

fn parse_cmd_line() -> Settings {
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
        .get_matches().to_owned();

    let mut settings = Settings {
        user_agent: match args.value_of("user-agent") {
            Some(ua) => ua.to_string(),
            None => env!("CARGO_PKG_VERSION").to_string()
        },
        rate_limit: args.value_of("rate-limit").unwrap().parse::<u64>().unwrap(),
        scoped: args.value_of("scoped").unwrap().parse::<bool>().unwrap(),
        recurse: match args.is_present("wordlist") {
            true => args.is_present("deep"),
            false => !args.is_present("shallow")
        },
        wordlist_filename: match args.value_of("wordlist") {
            Some(wl) => Some(wl.to_string()),
            None => None
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
            },
            None => None
        },
        status_include: match args.values_of_t::<u16>("status-include") {
            Ok(v) => v,
            Err(_) => vec![]
        },
        status_exclude: match args.values_of_t::<u16>("status-exclude") {
            Ok(v) => v,
            Err(_) => vec![]
        },
        verbosity: match args.occurrences_of("verbosity") {
            0 => Level::WARN,
            1 => Level::INFO,
            2 => Level::DEBUG,
            _ => Level::TRACE,
        },
        quiet: args.value_of_t::<bool>("quiet").unwrap(),
        hosts: get_hosts_from_args(args),
        flags: Flags::NONE,
        max_threads: 50
    };

    pre_configure(&mut settings);

    settings
}

fn pre_configure(settings: &mut Settings) {
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
fn exclude_not_found_if_force_browsing(settings: &mut Settings) {
    if !settings.recurse && settings.status_exclude.is_empty() {
        settings.status_exclude = vec![404];
    }
}

fn get_hosts_from_args(args: ArgMatches) -> Vec<String> {
    match args.values_of_lossy("host") {
        Some(hosts) => hosts,
        None => {
            let single_host =
                args.value_of("single_host").unwrap().to_string();
            let mut vec: Vec<String> = Vec::new();
            vec.push(single_host);
            vec
        }
    }
}

fn configure_logging(verbosity_level: Level) {
    tracing_subscriber::fmt().with_max_level(verbosity_level).init();
    info!("Verbosity level set to {}", verbosity_level);
    trace!("configured logging");
}

