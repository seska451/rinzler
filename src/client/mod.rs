use crate::config::RinzlerSettings;
use crate::crawler::crawl_target::CrawlTarget;
use reqwest::blocking::{Client, Response};
use reqwest::header::HeaderMap;
use reqwest::{Method, Result};
use std::sync::Arc;

pub struct RequestOptions {
    truncate: bool,
}
impl RequestOptions {
    pub const fn default() -> Option<RequestOptions> {
        Some(RequestOptions { truncate: false })
    }

    pub const fn with_partial_get() -> Option<RequestOptions> {
        Some(RequestOptions { truncate: true })
    }
}

pub struct RinzlerClient {
    client: Arc<Client>,
}

impl Clone for RinzlerClient {
    fn clone(&self) -> Self {
        RinzlerClient {
            client: Arc::clone(&self.client),
        }
    }
}

impl RinzlerClient {
    pub(crate) fn new(settings: &RinzlerSettings) -> RinzlerClient {
        RinzlerClient {
            client: RinzlerClient::create_http_client(settings),
        }
    }

    pub(crate) fn send_get(
        &self,
        ct: &mut CrawlTarget,
        opt: Option<RequestOptions>,
    ) -> Result<Response> {
        ct.method = Method::GET.to_string();

        let result = self.client.get(&ct.url).send();
        result
    }

    pub(crate) fn send_head(
        &self,
        ct: &mut CrawlTarget,
        opt: Option<RequestOptions>,
    ) -> Result<Response> {
        ct.method = Method::HEAD.to_string();
        let result = self.client.head(&ct.url).send();
        result
    }

    pub(crate) fn send_options(
        &self,
        crawl_target: &mut CrawlTarget,
        opt: Option<RequestOptions>,
    ) -> Result<Response> {
        crawl_target.method = Method::OPTIONS.to_string();

        let result = self
            .client
            .request(Method::OPTIONS, &crawl_target.url)
            .send();
        result
    }

    fn create_http_client(settings: &RinzlerSettings) -> Arc<Client> {
        let headers = HeaderMap::new();
        let client = reqwest::blocking::ClientBuilder::new()
            .user_agent(settings.user_agent.as_str())
            .danger_accept_invalid_certs(true)
            .default_headers(headers)
            //.timeout(Duration::from_millis(5000))
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .unwrap();

        Arc::new(client)
    }
}
