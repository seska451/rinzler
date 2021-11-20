use std::thread;
use std::time::Duration;
use regex::Regex;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::{Client, Error, Response, StatusCode};
use url::{Url};
use crate::{HeaderMap, HeaderValue, Settings, USER_AGENT};
use console::{style, StyledObject};

pub async fn run(settings: &Settings) -> Result<(), Box<dyn std::error::Error>> {
    let mut visited : Vec<Url> = Vec::new();                  // this tracks which locations have already been visited
    let mut scoped_domains: Vec<Url> = Vec::new();            // keep a list of scoped domains to restrict search scope later
    let mut to_visit : Vec<Url> = Vec::new();              // this stack contains all desired locations to visit
    let is_force_browse = !settings.recurse;
    init_visitees(settings, &mut to_visit, &mut scoped_domains);
    let pb = ProgressBar::new(to_visit.len() as u64);
    pb.set_style(pb_style_no_total());
    loop {
        let next_url = to_visit.remove(0);
        match Url::parse(next_url.as_str()) {
            Ok(url) => {
                let is_scoped_scan = settings.scoped.clone();

                if !get_has_been_visited(&mut visited, &url) && (is_scoped_scan && get_is_url_in_scope(&mut scoped_domains, &url) || !is_scoped_scan) {
                    crawl(url, &mut to_visit, &mut visited, &settings, &pb).await?;
                }
                if !is_force_browse { pb.inc(1) };
            }
            Err(_) => break
        }
        if to_visit.is_empty() { break }
    }
    pb.finish();
    Ok(())
}

async fn crawl(url: Url, to_visit: &mut Vec<Url>, visited: &mut Vec<Url>, settings: &Settings, pb: &ProgressBar) -> Result<(), Box<dyn std::error::Error>> {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_str(settings.user_agent.as_str()).unwrap());

    let response = get_response(&url, visited, &headers).await?;
    let code = response.status();
    pb.set_length(to_visit.len() as u64);
    if settings.recurse && is_code_filtered(u16::from(code), settings) {
        if settings.quiet {
            print_found_quiet(&url, code);
        } else {
            print_found_with_progress(&url, code, pb).unwrap();
        }
    }

    if settings.wordlist_filename.is_some() {
        if settings.quiet {
            forced_browse_quiet(&url, visited, settings, &mut headers).await;
        } else {
            forced_browse(&url, visited, settings, &mut headers).await;
        }
    }

    if settings.recurse {
        let body = response.text().await?;
        let url_finder : Regex = Regex::new("(?:src=[\"']|href=[\"'])(/{0,2}[^\"',<>]*)").unwrap();
        url_finder.captures_iter(body.as_str())
            .for_each(|captures|  {
                match captures.get(1) {
                    Some(u) => {
                        thread::sleep(Duration::from_millis(settings.rate_limit));
                        let part_url = url.join(u.as_str()).unwrap();
                        to_visit.push(part_url)
                    },
                    None => ()
                }
            });
    };

    Ok(())
}

async fn forced_browse_quiet(url: &Url, visited: &mut Vec<Url>, settings: &Settings, headers: &mut HeaderMap) {
    for word in settings.wordlist.clone().unwrap() {
        let url_to_check = url.join(word.as_str()).unwrap();
        let res = get_response(&url_to_check, visited, &headers).await;
        let status_code = res.unwrap().status();
        if is_code_filtered(u16::from(status_code), settings) {
            print_found_quiet(&url_to_check, status_code);
        }
    }
}

async fn forced_browse(url: &Url, visited: &mut Vec<Url>, settings: &Settings, headers: &mut HeaderMap) {
    let wordlist = settings.wordlist.clone().unwrap();
    let len = wordlist.len() as u64;
    let pb = ProgressBar::new(len);
    pb.set_style(pb_style_total());
    pb.set_length(len);
    for word in wordlist {
        let url_to_check = url.join(word.as_str()).unwrap();
        let res = get_response(&url_to_check, visited, &headers).await;
        let status_code = res.unwrap().status();
        if is_code_filtered(u16::from(status_code), settings) {
            print_found_with_progress(&url_to_check, status_code, &pb).unwrap();
        }
        pb.set_message(format!("{}", url_to_check));
        pb.inc(1);
    }
}

async fn get_response(url: &Url, visited: &mut Vec<Url>, headers: &HeaderMap) -> Result<Response, Error> {
    let client = Client::new();
    let res = client
        .get(url.to_owned())
        .headers(headers.to_owned())
        .send()
        .await?;
    visited.push(url.clone());
    Ok(res)
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

fn init_visitees(settings: &Settings, to_visit: &mut Vec<Url>, scoped_domains: &mut Vec<Url>) {
    settings.hosts.clone().into_iter().for_each(|u| {
        let url : Url = match Url::parse(u.as_str()) {
            Ok(u) => u,
            Err(why) => panic!("supplied scope wasn't a valid domain: {}", why)
        };
        if !scoped_domains.contains(&url) {
            scoped_domains.push(url.to_owned());
        }
        to_visit.push(url.to_owned());
    });
}

fn is_code_filtered(code : u16, settings: &Settings) -> bool{
    let to_inc = &settings.status_include;
    let inclusions_exist = to_inc.is_empty() == false;
    let to_exc = &settings.status_exclude;
    let exclusions_exist = to_exc.is_empty() == false;
    let code = &u16::from(code);
    let mut allow = true;

    if inclusions_exist {
        allow &= to_inc.contains(code);
    }

    if exclusions_exist {
        allow &= !to_exc.contains(code);
    }

    allow
}

fn pb_style_total() -> ProgressStyle {
    ProgressStyle::default_bar()
        .template("{spinner:.green} [{bar:40.cyan/blue}] [{elapsed_precise}] [{pos}/{len}] {msg}")
        .progress_chars("#>-")
}

fn pb_style_no_total() -> ProgressStyle {
    ProgressStyle::default_bar()
        .template("{spinner:.green} [{bar:40.cyan/blue}] [{elapsed_precise}] {msg}")
        .progress_chars("#>-")
}

fn print_found_with_progress(url: &Url, code: StatusCode, pb: &ProgressBar) -> Result<(), Box<dyn std::error::Error>> {
    let int_code = code.as_u16();
    let styled_code  = get_styled_statuscode(int_code);

    pb.println(&format!("[{}] {}", styled_code, style(url).green()));
    Ok(())
}

fn get_styled_statuscode(int_code: u16) -> StyledObject<u16> {
    match int_code {
        int_code if int_code < 299 => style(int_code).cyan(),
        int_code if int_code >= 300 && int_code < 399 => style(int_code).yellow(),
        int_code if int_code > 400 => style(int_code).red(),
        _ => style(int_code).white(),
    }
}

fn print_found_quiet(url: &Url, code: StatusCode) {
    let int_code = code.as_u16();
    let styled_code  = get_styled_statuscode(int_code);
    println!("[{}] {}", styled_code, style(url).green());
}