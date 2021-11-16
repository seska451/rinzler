use std::borrow::{Borrow, BorrowMut};
use std::env;
use std::error::Error;
use std::string::ParseError;
use clap::{Arg, App, ArgMatches};
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use reqwest::Client;
use url::{Url};
use tracing::{info, warn, debug, trace, error, Level};
use tracing_subscriber;
use tracing_subscriber::fmt::SubscriberBuilder;
use select::document::Document;
use select::predicate::Name;
use std::thread;
use std::time::Duration;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use queues::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = App::new("rinzler")
        .version(env!("CARGO_PKG_VERSION"))
        .author("seska <seska@seska.io>")
        .about("A simple to use, multithreaded web crawler written in rustlang.")
        .arg(Arg::new("host")
            .short('h')
            .long("host")
            .value_name("HOST URL")
            .required(true)
            .multiple_occurrences(true)
            .env("RINZLER_HOSTS")
            .takes_value(true)
            .about("Set the initial URL to start crawling. Can be set multiple times to crawl several sites at once."))
        .arg(Arg::new("verbosity")
            .short('v')
            .multiple_occurrences(true)
            .about("Sets the level of output verbosity. Set multiple times "))
        .arg(Arg::new("format")
            .short('f')
            .long("format")
            .value_name("OUTPUT FORMAT")
            .env("RINZLER_FORMAT")
            .about("Controls the type of output. Supports 'txt' & 'json', defaults to 'txt'."))
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

        .get_matches();

    if !matches.is_present("quiet") {
        print_banner();
    }

    let subscriber_builder = tracing_subscriber::fmt();
    configure_logging(&matches, subscriber_builder);
    let ua =  matches.borrow().value_of("user-agent").unwrap();
    let ratelimit_string = matches.borrow().value_of("rate-limit").unwrap();
    let ratelimit = ratelimit_string.parse::<u64>().unwrap();
    let scoped_hosts = matches.values_of_lossy("host").unwrap().into_iter();
    let scoped = matches.borrow().value_of("scoped").unwrap().parse::<bool>().unwrap();

    let mut visit_queue = queues::Queue::new();
    let mut visited : Vec<Url> = Vec::new();
    scoped_hosts.clone().for_each(|u| {
        visit_queue.add(u.clone());
    });

    let mut scoped_domains: Vec<Url> =
        scoped_hosts.clone().map(|host|
            Url::parse(host.as_str()).unwrap()).collect();

    loop {
        let url = visit_queue.remove().unwrap();
        let url_str = url.as_str();

        match Url::parse(url_str) {
            Ok(u) => {
                let is_in_scope = scoped_domains.iter().map(|x| x.domain()).into_iter().any(|y| y.unwrap() == u.domain().unwrap());
                let has_been_visited = visited.iter().any(|y| y == &u);
                if !has_been_visited && ( scoped && is_in_scope || !scoped) {
                    //if we've set limit and the new url domain is in scope, or there are no limits
                    //then visit the url
                    visit_url(url, &mut visit_queue, &mut visited, &ua, ratelimit, scoped).await?
                }
            },
            Err(why) => error!("{}", why)
        }
        if visit_queue.size() == 0 { break; }
    }

    Ok(())
}

async fn visit_url(url: String, q: &mut Queue<String>, visited: &mut Vec<Url>, ua: &str, ratelimit: u64, limit: bool) -> Result<(), Box<dyn std::error::Error>> {
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
             q.add(String::from(to_visit));
        });
    Ok(())
}

fn configure_logging(matches: &ArgMatches, subscriber_builder: SubscriberBuilder) {
    let verbosity_level = matches.occurrences_of("verbosity");
    let builder = match verbosity_level {
        0 => subscriber_builder.with_max_level(Level::WARN),
        1 => subscriber_builder.with_max_level(Level::INFO),
        2 => subscriber_builder.with_max_level(Level::DEBUG),
        _ => subscriber_builder.with_max_level(Level::TRACE),
    };

    builder.init();
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
    println!("🙌    rinzler --url URL     🙌");
    println!("🙌                          🙌");
}

