use std::thread;
use std::time::Duration;
use regex::Regex;
use reqwest::Client;
use tracing::{error, info};
use url::Url;
use crate::{HeaderMap, HeaderValue, Settings, USER_AGENT};

pub async fn run(settings: &Settings) -> Result<(), Box<dyn std::error::Error>> {
    let mut visited : Vec<Url> = Vec::new();                  // this tracks which locations have already been visited
    let mut scoped_domains: Vec<Url> = Vec::new();            // keep a list of scoped domains to restrict search scope later
    let mut to_visit : Vec<String> = Vec::new();              // this stack contains all desired locations to visit

    init_visitees(settings, &mut to_visit, &mut scoped_domains);

    loop {
        //commence the crawl
        //pop the next url to visit off the queue
        let next_url = to_visit.remove(0);

        //parse the url
        match Url::parse(next_url.as_str()) {
            Ok(url) => {
                let is_scoped_scan = settings.scoped.clone();

                //if we're doing a scoped scan and the new url domain is in scope
                //if we're doing an unscoped scan
                if !get_has_been_visited(&mut visited, &url) && (is_scoped_scan && get_is_url_in_scope(&mut scoped_domains, &url) || !is_scoped_scan) {
                    //then visit the url
                    crawl(url.to_string(), &mut to_visit, &mut visited, &settings).await?
                }
            }
            Err(why) => error!("{}", why)
        }
        if to_visit.is_empty() { break }
    }
    Ok(())
}

fn get_has_been_visited(visited: &mut Vec<Url>, url: &Url) -> bool {
    visited.iter().any(|y| y == url)
}

fn get_is_url_in_scope(scoped_domains: &mut Vec<Url>, url: &Url) -> bool {
    scoped_domains.iter()
        .map(|u| match u.domain() {
            Some(domain) => domain,
            None => panic!("invalid scoped domain")
        }).into_iter()
        .any(|domain|  domain == url.domain().unwrap_or_default())
}

fn init_visitees(settings: &Settings, to_visit: &mut Vec<String>, scoped_domains: &mut Vec<Url>) {
    settings.hosts.clone().into_iter().for_each(|u| {
        let url : Url = match Url::parse(u.as_str()) {
            Ok(u) => u,
            Err(why) => panic!("supplied scope wasn't a valid domain: {}", why)
        };
        if !scoped_domains.contains(&url) {
            scoped_domains.push(url);
        }
        to_visit.push(u.clone());
    });
}


async fn crawl(url: String, to_visit: &mut Vec<String>, visited: &mut Vec<Url>, settings: &Settings) -> Result<(), Box<dyn std::error::Error>> {
    info!("Reading URL {}", &url);
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_str(settings.user_agent.as_str()).unwrap());
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

    if settings.recurse {
        let url_finder : Regex = Regex::new("(?:src=[\"']|href=[\"'])(/{0,2}[^\"',<>]*)").unwrap();
        url_finder.captures_iter(body.as_str())
            .for_each(|captures|  {
                match captures.get(1) {
                    Some(u) => {
                        info!("🔍 Found URL: {}", u.as_str());
                        thread::sleep(Duration::from_millis(settings.rate_limit));
                        let part_url = url_parsed.join(u.as_str()).unwrap();
                        to_visit.push(String::from(part_url))
                    },
                    None => ()
                }

            });
    }

    Ok(())
}
