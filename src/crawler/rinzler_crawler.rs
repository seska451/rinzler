use std::sync::{Arc, Mutex};
use crossbeam::channel::{Sender};
use std::thread;
use std::time::Duration;
use chrono::Local;
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use reqwest::{ Result};
use reqwest::blocking::Response;
use url::{ParseError, Url};
use crate::crawler::crawl_target::CrawlTarget;
use crate::{ConsoleMessage, ConsoleMessageType, Settings};
use rayon::prelude::*;

pub enum ControllerMessageType {
    FINISHED,
    ERROR
}

pub struct ControllerMessage {
    pub message_type: ControllerMessageType,
    pub data: String
}

pub struct RinzlerCrawler {
    target: String,
    settings: Settings,
    pub controller_sender: Sender<ControllerMessage>,
    pub console_sender: Sender<ConsoleMessage>,
    scoped_domains: Vec<String>
}

impl RinzlerCrawler {
    pub(crate) fn finish(&self) {
        let _ = self.controller_sender.send(ControllerMessage {
            message_type: ControllerMessageType::FINISHED,
            data: "".to_string()
        });
    }
}

impl RinzlerCrawler {
    pub fn new(target: String, settings : Settings, controller_messages: Sender<ControllerMessage>, console_messages: Sender<ConsoleMessage>, scoped_domains: Vec<String>) -> RinzlerCrawler {
        RinzlerCrawler {
            target,
            settings: settings.to_owned(),
            controller_sender: controller_messages,
            console_sender: console_messages,
            scoped_domains
        }
    }

    pub(crate) fn crawl(&self, already_visited: Arc<std::sync::Mutex<Vec<String>>>) -> Result<()> {
        let wordlist = &self.settings.wordlist;
        let target = &self.target;
        let mut crawl_target = CrawlTarget::new();

        match Url::parse(&target) {
            Ok(u) => {
                crawl_target.url = u.to_string();
                self.send_target_found_message(&mut crawl_target);
            },
            Err(why) => { //we dont want to continue when bogus urls are supplied
                self.send_abort_program_message(&target, why);
                return Ok(());
            }
        };

        self.force_browse(&already_visited, crawl_target.clone(), wordlist.to_owned());
        self.find_new_urls(&already_visited, crawl_target.clone());
        Ok(())
    }

    fn send_abort_program_message(&self, target: &&String, why: ParseError) {
        let _ = self.console_sender.send(ConsoleMessage {
            message_type: ConsoleMessageType::ABORT,
            data: Err(format!("Couldn't parse '{}' as a URL: {}", &target, why)),
            crawl_target: None,
        });
    }

    fn send_target_found_message(&self, crawl_target: &mut CrawlTarget) {
        let _ = self.console_sender.send(ConsoleMessage {
            message_type: ConsoleMessageType::RESULT,
            data: Ok(String::default()),
            crawl_target: Some(crawl_target.clone()),
        });
    }

    fn find_new_urls(&self, visited: &Arc<Mutex<Vec<String>>>, crawl_target: CrawlTarget) {

        let url_str = crawl_target.url.clone();
        let url = Url::parse(url_str.as_str()).unwrap();
        let result = self.get_response(&url_str);

        if let Ok(res) = result {
            self.send_target_hit_message(visited, crawl_target, &res);

            let content_type = res.headers().get(reqwest::header::CONTENT_TYPE).unwrap();
            let content_type = content_type.to_str().unwrap_or_default();
            if !content_type.contains("text/") {
                //skip any content that we cant use regex over
                return;
            }

            if let Ok(body) = res.text() {
                let url_finder: Regex = Regex::new("(?:src=[\"']|href=[\"'])(/{0,2}[^\"',<>]*)").unwrap();
                url_finder.captures_iter(body.as_str())
                    .for_each(|captures| {
                        match captures.get(1) {
                            Some(u) => {
                                let part_url = &url.join(u.as_str()).unwrap();
                                if !visited.lock().unwrap().contains(&part_url.to_string()) {
                                    let target_domain = &part_url.domain().unwrap_or_default().to_string();
                                    if !self.settings.scoped || (self.settings.scoped && self.scoped_domains.contains(target_domain))  {
                                        self.recurse(&visited, part_url);
                                    }
                                }
                            },
                            None => ()
                        }
                    });
            }
        }
    }

    fn send_target_hit_message(&self, visited: &Arc<Mutex<Vec<String>>>, mut crawl_target: CrawlTarget, res: &Response) {
        visited.lock().unwrap().push(crawl_target.url.clone());
        crawl_target.url = res.url().to_string();
        crawl_target.status_code = Some(u16::from(res.status()));
        crawl_target.timestamp = Local::now();

        let _ = self.console_sender.send(ConsoleMessage {
            message_type: ConsoleMessageType::RESULT,
            data: Ok(String::default()),
            crawl_target: Some(crawl_target.clone()),
        });
    }

    fn get_response(&self, url_str: &String) -> Result<Response> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_str(self.settings.user_agent.as_str()).unwrap());
        let headers_argument = &headers;
        let client = reqwest::blocking::Client::new();
        let result = client
            .get(url_str)
            .headers(headers_argument.to_owned())
            .send();
        result
    }

    fn recurse(&self, visited: &Arc<Mutex<Vec<String>>>, part_url: &Url) {
        let new_crawl = RinzlerCrawler {
            target: part_url.to_string(),
            settings: self.settings.clone(),
            controller_sender: self.controller_sender.clone(),
            console_sender: self.console_sender.clone(),
            scoped_domains: self.scoped_domains.clone(),
        };
        thread::sleep(Duration::from_millis(self.settings.rate_limit));
        let _ = new_crawl.crawl(Arc::clone(&visited));
    }

    fn force_browse(&self, visited: &Arc<Mutex<Vec<String>>>, crawl_target: CrawlTarget, wordlist: Option<Vec<String>>) {
        if let Ok(base_url) = Url::parse(crawl_target.url.as_str()) {
            let wl = wordlist.unwrap();
            wl.par_iter().for_each(|word| {
                if let Ok(to_visit) = base_url.join(word.as_str()) {
                    let mut crawl_target = CrawlTarget::from_url(to_visit.clone());
                    self.send_target_found_message(&mut crawl_target);
                    let result = self.get_response(&to_visit.to_string());
                    let response = result.unwrap();
                    let status_code = response.status();
                    if self.is_allowed(u16::from(status_code)) {
                        self.send_target_hit_message(visited, crawl_target.clone(),&response)
                    }
                }
            });
        }
    }

    fn is_allowed(&self, code : u16) -> bool {
        let allowed_status_codes = self.settings.status_include.to_owned();
        let blocked_status_codes = self.settings.status_exclude.to_owned();

        let inclusions_exist = allowed_status_codes.is_empty() == false;
        let exclusions_exist = blocked_status_codes.is_empty() == false;
        let code = &u16::from(code);
        let mut allow = true;

        if inclusions_exist {
            allow &= allowed_status_codes.contains(code);
        }

        if exclusions_exist {
            allow &= !blocked_status_codes.contains(code);
        }

        allow
    }
}