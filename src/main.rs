use std::env;
use clap::{Arg, App, ArgMatches};
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use tracing::{info, trace, Level};
use tracing_subscriber;
use std::vec::IntoIter;
use config::Settings;
use stopwatch::Stopwatch;
mod crawler;
mod config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = parse_cmd_line();
    if !settings.quiet {
        print_banner();
    }
    configure_logging(settings.verbosity);
    let sw = Stopwatch::start_new();
    crawler::run(&settings).await?;
    println!("Crawl execution time {}s", sw.elapsed().as_secs());
    Ok(())
}

fn parse_cmd_line() -> Settings {
    let args = App::new("rinzler")
        .version(env!("CARGO_PKG_VERSION"))
        .author("seska <seska@seska.io>")
        .about("A simple to use, multithreaded web crawler written in rustlang.")
        .arg(Arg::new("single_host")
                 .index(1)
                 .conflicts_with("host")
                 .required(true)
                 .value_name("HOST URL")
                 .about("The host URL to scan"))
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
            .env("RINZLER_RATELIMIT")
            .takes_value(true)
            .default_value("0")
            .about("Set the number of milliseconds to wait between each request."))
        .get_matches().to_owned();

    Settings {
        user_agent: match args.value_of("user-agent") {
            Some(ua) => ua.to_string(),
            None => env!("CARGO_PKG_VERSION").to_string()
        },
        rate_limit: args.value_of("rate-limit").unwrap().parse::<u64>().unwrap(),
        scoped: args.value_of("scoped").unwrap().parse::<bool>().unwrap(),
        verbosity: match args.occurrences_of("verbosity") {
            0 => Level::WARN,
            1 => Level::INFO,
            2 => Level::DEBUG,
            _ => Level::TRACE,
        },
        hosts: get_hosts_from_args(args),
        quiet: false,
    }
}

fn get_hosts_from_args(args: ArgMatches) -> IntoIter<String> {
    match args.values_of_lossy("host") {
        Some(hosts) => hosts.into_iter(),
        None => {
            let single_host =
                args.value_of("single_host").unwrap().to_string();
            let mut vec: Vec<String> = Vec::new();
            vec.push(single_host);
            vec.into_iter()
        }
    }
}

fn configure_logging(verbosity_level: Level) {
    tracing_subscriber::fmt().with_max_level(verbosity_level).init();
    info!("Verbosity level set to {}", verbosity_level);
    trace!("configured logging");
}

fn print_banner() {
    let ver = env!("CARGO_PKG_VERSION");
    println!("         _             __");
    println!("   _____(_)___  ____  / /__  _____");
    println!("  / ___/ / __ \\/_  / / / _ \\/ ___/");
    println!(" / /  / / / / / / /_/ /  __/ /");
    println!("/_/  /_/_/ /_/ /___/_/\\___/_/");
    println!("         v{}        ", ver);
    println!("🙌   a fast webcrawler      🙌");
    println!("🙌   from seska with ♡♡♡    🙌");
    println!("🙌                          🙌");
    println!("🙌   usage: rinzler <URL>   🙌");
    println!("🙌                          🙌");
}