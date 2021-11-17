use std::thread;
use std::time::Duration;
use queues::{IsQueue, Queue};
use reqwest::Client;
use select::document::Document;
use select::predicate::Name;
use tracing::{debug, error, info};
use url::Url;
use crate::{HeaderMap, HeaderValue, Settings, USER_AGENT};

pub async fn run(settings: &Settings) -> Result<(), Box<dyn std::error::Error>> {
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
