use crate::config::{Flags, RinzlerSettings};
use crate::crawler::crawl_target::CrawlTarget;
use crate::ui::rinzler_console::{ConsoleMessage, ConsoleMessageType};
use chrono::Local;
use crossbeam::channel::Sender;
use rayon::prelude::*;
use regex::Regex;
use reqwest::blocking::{Client, Response};
use reqwest::header::{HeaderMap, HeaderValue, RANGE};
use reqwest::{Method, Result};
use std::sync::{Arc, Mutex};
use tracing::{debug, info};
use url::{ParseError, Url};

struct RequestOptions {
    truncate: bool,
}
impl RequestOptions {
    const fn default() -> Option<RequestOptions> {
        Some(RequestOptions { truncate: false })
    }

    const fn with_partial_get() -> Option<RequestOptions> {
        Some(RequestOptions { truncate: true })
    }
}

pub enum ControllerMessageType {
    FINISHED,
    ERROR,
}

pub struct ControllerMessage {
    pub message_type: ControllerMessageType,
    pub data: String,
}

pub struct RinzlerCrawler {
    target: String,
    settings: RinzlerSettings,
    pub controller_sender: Sender<ControllerMessage>,
    pub console_sender: Sender<ConsoleMessage>,
    scoped_domains: Vec<String>,
}

impl RinzlerCrawler {
    pub(crate) fn finish(&self) {
        let _ = self.controller_sender.send(ControllerMessage {
            message_type: ControllerMessageType::FINISHED,
            data: "".to_string(),
        });
    }
}

impl RinzlerCrawler {
    pub fn new(
        target: String,
        settings: RinzlerSettings,
        controller_messages: Sender<ControllerMessage>,
        console_messages: Sender<ConsoleMessage>,
        scoped_domains: Vec<String>,
    ) -> RinzlerCrawler {
        RinzlerCrawler {
            target,
            settings: settings.to_owned(),
            controller_sender: controller_messages,
            console_sender: console_messages,
            scoped_domains,
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
            }
            Err(why) => {
                //we dont want to continue when bogus urls are supplied
                self.send_abort_program_message(&target, why);
                return Ok(());
            }
        };
        let flags = &self.settings.flags;
        if let Some(wordlist) = wordlist {
            if flags.contains(Flags::BRUTE) {
                self.force_browse(&already_visited, crawl_target.clone(), wordlist.to_owned());
            }
        }
        if flags.contains(Flags::CRAWL) {
            self.find_new_urls(&already_visited, crawl_target.clone());
        }
        Ok(())
    }

    fn send_abort_program_message(&self, target: &&String, why: ParseError) {
        let _ = self.console_sender.send(ConsoleMessage {
            message_type: ConsoleMessageType::Abort,
            data: Err(format!("Couldn't parse '{}' as a URL: {}", &target, why)),
            original_target: None,
            crawl_target: None,
            total: None,
        });
    }

    fn send_target_found_message(&self, crawl_target: &mut CrawlTarget) {
        let _ = self.console_sender.send(ConsoleMessage {
            message_type: ConsoleMessageType::Result,
            data: Ok(String::default()),
            original_target: None,
            crawl_target: Some(crawl_target.clone()),
            total: None,
        });
    }

    fn find_new_urls(&self, visited: &Arc<Mutex<Vec<String>>>, crawl_target: CrawlTarget) {
        let url_str = crawl_target.url.clone();
        let url = Url::parse(url_str.as_str()).unwrap();

        let result = self.send_head(&url_str, RequestOptions::default());

        if let Ok(res) = result {
            self.send_target_hit_message(visited, crawl_target, &res);

            let content_type = res.headers().get(reqwest::header::CONTENT_TYPE).unwrap();
            let content_type = content_type.to_str().unwrap_or_default();
            if !content_type.contains("text/") {
                return;
            }
            match self.send_get(&url_str, RequestOptions::with_partial_get()) {
                Ok(res) => {
                    if let Ok(body) = res.text() {
                        let url_finder: Regex =
                            Regex::new("(?:src=[\"']|href=[\"'])(/{0,2}[^\"',<>]*)").unwrap();
                        url_finder
                            .captures_iter(body.as_str())
                            .for_each(|captures| match captures.get(1) {
                                Some(u) => {
                                    let part_url = &url.join(u.as_str()).unwrap();
                                    if !visited.lock().unwrap().contains(&part_url.to_string()) {
                                        let target_domain =
                                            &part_url.domain().unwrap_or_default().to_string();
                                        if !self.settings.scoped
                                            || (self.settings.scoped
                                                && self.scoped_domains.contains(target_domain))
                                        {
                                            self.recurse(&visited, part_url);
                                        }
                                    }
                                }
                                None => (),
                            });
                    }
                }
                Err(_) => {}
            }
        }
    }

    fn send_target_hit_message(
        &self,
        visited: &Arc<Mutex<Vec<String>>>,
        mut crawl_target: CrawlTarget,
        res: &Response,
    ) {
        visited.lock().unwrap().push(crawl_target.url.clone());
        crawl_target.url = res.url().to_string();
        crawl_target.status_code = Some(u16::from(res.status()));
        crawl_target.timestamp = Local::now();

        let _ = self.console_sender.send(ConsoleMessage {
            message_type: ConsoleMessageType::Result,
            data: Ok(String::default()),
            original_target: None,
            crawl_target: Some(crawl_target.clone()),
            total: None,
        });
    }

    fn send_get(&self, url_str: &String, truncate: Option<RequestOptions>) -> Result<Response> {
        let client = self.get_http_client(truncate);

        let result = client.get(url_str).send();
        result
    }

    fn send_head(&self, url_str: &String, truncate: Option<RequestOptions>) -> Result<Response> {
        let client = self.get_http_client(truncate);

        let result = client.head(url_str).send();
        result
    }

    fn send_options(&self, url_str: &String, truncate: Option<RequestOptions>) -> Result<Response> {
        let client = self.get_http_client(truncate);

        let result = client.request(Method::OPTIONS, url_str).send();
        result
    }

    fn get_http_client(&self, truncate: Option<RequestOptions>) -> Client {
        let mut headers = HeaderMap::new();
        if let Some(opt) = truncate {
            if opt.truncate {
                headers.insert(RANGE, HeaderValue::from_str("-100").unwrap());
            }
        }

        let client = reqwest::blocking::ClientBuilder::new()
            .user_agent(self.settings.user_agent.as_str())
            .danger_accept_invalid_certs(true)
            .default_headers(headers)
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .unwrap();
        client
    }

    fn recurse(&self, visited: &Arc<Mutex<Vec<String>>>, part_url: &Url) {
        let new_crawl = RinzlerCrawler {
            target: part_url.to_string(),
            settings: self.settings.clone(),
            controller_sender: self.controller_sender.clone(),
            console_sender: self.console_sender.clone(),
            scoped_domains: self.scoped_domains.clone(),
        };
        let _ = new_crawl.crawl(Arc::clone(&visited));
    }

    fn force_browse(
        &self,
        visited: &Arc<Mutex<Vec<String>>>,
        crawl_target: CrawlTarget,
        mut wordlist: Vec<String>,
    ) {
        if let Ok(base_url) = Url::parse(crawl_target.url.as_str()) {
            self.send_start_force_browse_message(wordlist.len(), crawl_target.clone());
            wordlist.par_iter().for_each(|word| {
                if let Ok(to_visit) = base_url.join(word.as_str()) {
                    let new_crawl_target = CrawlTarget::from_url(to_visit.clone());
                    self.send_force_browse_attempt(new_crawl_target.clone(), crawl_target.clone());
                    let result = self.send_head_or_get(&to_visit);

                    match result {
                        Ok(response) => {
                            let status_code = response.status();
                            if self.is_allowed(u16::from(status_code)) {
                                self.send_force_browse_hit(visited, crawl_target.clone(), &response)
                            }
                        }
                        Err(_) => { /* probably nothing to do here */ }
                    }
                    self.send_force_browse_progress(crawl_target.clone());
                }
            });
        }
    }

    fn send_head_or_get(&self, to_visit: &Url) -> Result<Response> {
        let result = self.send_head(&to_visit.to_string(), RequestOptions::default());

        match result {
            Ok(r) => match r.status().as_u16() {
                500..=599 => {
                    self.send_get(&to_visit.to_string(), RequestOptions::with_partial_get())
                }
                _ => Ok(r),
            },
            Err(_) => result,
        }
    }

    fn is_allowed(&self, code: u16) -> bool {
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
    fn send_start_force_browse_message(&self, len: usize, ct: CrawlTarget) {
        let _ = self.console_sender.send(ConsoleMessage {
            message_type: ConsoleMessageType::ForceBrowseStart,
            data: Ok(String::default()),
            original_target: None,
            crawl_target: Some(ct),
            total: Some(len as u64),
        });
    }
    fn send_force_browse_progress(&self, ct: CrawlTarget) {
        let _ = self.console_sender.send(ConsoleMessage {
            message_type: ConsoleMessageType::ForceBrowseProgress,
            data: Ok(String::default()),
            original_target: None,
            crawl_target: Some(ct),
            total: None,
        });
    }
    fn send_force_browse_hit(
        &self,
        visited: &Arc<Mutex<Vec<String>>>,
        mut ct: CrawlTarget,
        response: &Response,
    ) {
        visited.lock().unwrap().push(ct.url.to_string());
        ct.url = response.url().to_string();
        ct.status_code = Some(u16::from(response.status()));
        ct.timestamp = Local::now();

        let _ = self.console_sender.send(ConsoleMessage {
            message_type: ConsoleMessageType::ForceBrowseHit,
            data: Ok(String::default()),
            original_target: None,
            crawl_target: Some(ct.clone()),
            total: None,
        });
    }
    fn send_force_browse_attempt(&self, new_crawl_target: CrawlTarget, crawl_target: CrawlTarget) {
        let _ = self.console_sender.send(ConsoleMessage {
            message_type: ConsoleMessageType::ForceBrowseAttempt,
            data: Ok(String::default()),
            original_target: Some(crawl_target.clone()),
            crawl_target: Some(new_crawl_target.clone()),
            total: None,
        });
    }
}
