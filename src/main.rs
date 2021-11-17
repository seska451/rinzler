use std::env;
use clap::{Arg, App, ArgMatches};
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use reqwest::Client;
use url::{Url};
use tracing::{info, debug, trace, error, Level};
use tracing_subscriber;
use select::document::Document;
use select::predicate::Name;
use std::thread;
use std::time::Duration;
use std::vec::IntoIter;
use queues::*;

struct Settings {
    user_agent: String,
    rate_limit: u64,
    scoped: bool,
    verbosity: Level,
    quiet: bool,
    hosts: IntoIter<String>
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = parse_cmd_line();
    if !settings.quiet {
        print_banner();
    }
    configure_logging(settings.verbosity);
    run(&settings).await?;
    Ok(())
}

async fn run(settings: &Settings) -> Result<(), Box<dyn std::error::Error>> {
    let mut visit_queue = queues::Queue::new(); // this queue contains all desired locations to visit
    let mut visited : Vec<Url> = Vec::new();                  // this tracks which locations have already been visited
    let mut scoped_domains: Vec<Url> = Vec::new();            // keep a list of scoped domains to restrict search scope later

    initialize_visitation_queue(settings, &mut visit_queue, &mut scoped_domains);

    loop {
        //commence the crawl
        //pop the next url to visit off the queue
        let next_url = match visit_queue.remove() {
            Ok(next) => next,
            Err(why) => {
                error!("{}", why);
                break
            }
        };

        //parse the url
        match Url::parse(next_url.as_str()) {
            Ok(url) => {
                let is_scoped_scan = settings.scoped.clone();

                //if we're doing a scoped scan and the new url domain is in scope
                //if we're doing an unscoped scan
                if !get_has_been_visited(&mut visited, &url) && (is_scoped_scan && get_is_url_in_scope(&mut scoped_domains, &url) || !is_scoped_scan) {
                    //then visit the url
                    crawl(url.to_string(), &mut visit_queue, &mut visited, &settings.user_agent, settings.rate_limit.clone()).await?
                }
            }
            Err(why) => error!("{}", why)
        }
        if visit_queue.size() == 0 { break }
    }
    Ok(())
}

fn get_has_been_visited(visited: &mut Vec<Url>, url: &Url) -> bool {
    visited.iter().any(|y| y == url)
}

fn get_is_url_in_scope(scoped_domains: &mut Vec<Url>, url: &Url) -> bool {
    scoped_domains.iter().map(|x| x.domain()).into_iter().any(|y| y.unwrap() == url.domain().unwrap())
}

fn initialize_visitation_queue(settings: &Settings, visit_queue: &mut Queue<String>, scoped_domains: &mut Vec<Url>) {
    settings.hosts.clone().for_each(|u| {
        scoped_domains.push(Url::parse(u.as_str()).unwrap());
        match visit_queue.add(u.clone()) {
            Ok(_) => (),
            Err(why) => error!("{}", why),
        };
    });
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

async fn crawl(url: String, q: &mut Queue<String>, visited: &mut Vec<Url>, ua: &str, ratelimit: u64) -> Result<(), Box<dyn std::error::Error>> {
    info!("Reading URL {}", &url);
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_str(ua).unwrap());
    let url_parsed = Url::parse(url.as_str()).unwrap();

    let client = Client::new();
    let res = client
        .get(url)
        .headers(headers)
        .send()
        .await?;
    visited.push(url_parsed.clone());
    println!("{}: {}", res.status(), res.url());
    let body = res.text().await?;

    Document::from(body.as_str())
        .select(Name("a"))
        .filter_map(|n| n.attr("href"))
        .for_each(|href|  {
            debug!("🔍 Found URL: {}", href);
            thread::sleep(Duration::from_millis(ratelimit));
            let to_visit = url_parsed.join(href).unwrap();
             match q.add(String::from(to_visit)) {
                Ok(_) => (),
                Err(why) => error!("{}", why)
             }
        });
    Ok(())
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

