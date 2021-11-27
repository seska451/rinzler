use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;
use chrono::Local;
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, USER_AGENT};
use reqwest::{ Result};
use url::Url;
use crate::crawler::crawl_target::CrawlTarget;
use crate::{ConsoleMessage, ConsoleMessageType, Settings};

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
        let target = &self.target;
        let url_parse_result = Url::parse(&target);
        let mut crawl_target = CrawlTarget::new();

        let url = match url_parse_result {
            Ok(u) => {
                crawl_target.url = u.to_string();
                let _ = self.console_sender.send(ConsoleMessage {
                    message_type: ConsoleMessageType::RESULT,
                    data: Ok(String::default()),
                    crawl_target: Some(crawl_target.clone()),
                });
                u
            },
            Err(why) => {
                let _ = self.console_sender.send(ConsoleMessage {
                    message_type: ConsoleMessageType::ABORT,
                    data: Err(format!("Couldn't parse '{}' as a URL: {}", &target, why)),
                    crawl_target: None,
                });
                return Ok(());
            }
        };

        self.find_new_urls(&already_visited, crawl_target.clone(), url.clone());
        Ok(())
    }

    fn find_new_urls(&self, visited: &Arc<Mutex<Vec<String>>>, mut crawl_result: CrawlTarget, url: Url) {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_str(self.settings.user_agent.as_str()).unwrap());
        let headers_argument = &headers;
        let client = reqwest::blocking::Client::new();
        if let Ok(res) = client
            .get(url.clone())
            .headers(headers_argument.to_owned())
            .send() {

            visited.lock().unwrap().push(url.clone().to_string());
            crawl_result.url = res.url().to_string();
            crawl_result.status_code = Some(u16::from(res.status()));
            crawl_result.timestamp = Local::now();

            let _ = self.console_sender.send(ConsoleMessage {
                message_type: ConsoleMessageType::RESULT,
                data: Ok(String::default()),
                crawl_target: Some(crawl_result.clone()),
            });

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
                                    if self.settings.scoped && self.scoped_domains.contains(target_domain) {
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
}