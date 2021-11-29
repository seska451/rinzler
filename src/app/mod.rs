use crate::config::RinzlerSettings;
use crate::crawler::rinzler_crawler::{ControllerMessage, ControllerMessageType, RinzlerCrawler};
use crate::ui::rinzler_console::{ConsoleMessage, ConsoleMessageType, RinzlerConsole};
use crossbeam::channel::{unbounded, Receiver, Sender};
use rayon::ThreadPoolBuilder;
use std::error::Error;
use std::sync::{Arc, Mutex};
use threadpool::ThreadPool;
use url::Url;

pub(crate) struct RinzlerApplication {
    settings: RinzlerSettings,
}

impl RinzlerApplication {
    pub fn from_settings(settings: RinzlerSettings) -> RinzlerApplication {
        ThreadPoolBuilder::new()
            .thread_name(|i: usize| format!("rinzler-{}", i))
            .num_threads(45)
            .build_global()
            .unwrap();

        RinzlerApplication { settings }
    }

    pub async fn run(&self) -> Result<(), Box<dyn Error>> {
        let (console_sender, console_receiver) = unbounded();
        let settings = self.settings.clone();
        let thread_pool = threadpool::ThreadPool::new(settings.max_threads);

        RinzlerApplication::start_console(console_receiver, &thread_pool, settings.clone()).await?;

        let mut controller_receivers = vec![];
        let visited = Arc::new(Mutex::new(vec![]));
        let scoped_domains: Vec<String> = settings
            .hosts
            .iter()
            .map(|h| Url::parse(h).unwrap().domain().unwrap().to_string())
            .collect();

        RinzlerApplication::start_crawlers(
            settings.clone(),
            console_sender.clone(),
            &thread_pool,
            settings.hosts.clone(),
            &mut controller_receivers,
            visited,
            scoped_domains.clone(),
        );

        let outcome = RinzlerApplication::wait_for_crawlers_to_finish(&mut controller_receivers);

        RinzlerApplication::inform_console_to_exit(outcome, console_sender.clone());

        thread_pool.join();
        Ok(())
    }

    fn inform_console_to_exit(reason: Result<String, String>, command_tx: Sender<ConsoleMessage>) {
        let _ = command_tx.send(ConsoleMessage {
            message_type: ConsoleMessageType::Finish,
            data: reason,
            original_target: None,
            crawl_target: None,
            total: None,
        });
    }

    fn wait_for_crawlers_to_finish(
        controller_receivers: &mut Vec<Receiver<ControllerMessage>>,
    ) -> Result<String, String> {
        let mut errors = vec![];
        loop {
            let finished = controller_receivers.iter_mut().all(|r| {
                if let Ok(fin) = r.recv() {
                    match fin.message_type {
                        ControllerMessageType::FINISHED => true,
                        ControllerMessageType::ERROR => {
                            errors.push(fin.data);
                            true
                        }
                    }
                } else {
                    false
                }
            });

            if !errors.is_empty() {
                break;
            }
            if finished {
                break;
            }
        }

        if errors.is_empty() {
            Ok("Scan Completed".to_string())
        } else {
            let _ = format!("{}", errors.to_owned().join("\n")).as_str();
            Err("Scan Failed".to_string())
        }
    }

    fn start_crawlers(
        settings: RinzlerSettings,
        console_sender: Sender<ConsoleMessage>,
        thread_pool: &ThreadPool,
        hosts: Vec<String>,
        controller_receivers: &mut Vec<Receiver<ControllerMessage>>,
        visited: Arc<Mutex<Vec<String>>>,
        scoped_domains: Vec<String>,
    ) {
        for target in hosts {
            let settings = settings.clone();
            let (controller_sender, controller_receiver) = unbounded();
            let console_sender = console_sender.clone();
            let v = Arc::clone(&visited);
            let scoped_domains = scoped_domains.clone();
            thread_pool.execute(move || {
                let crawler = RinzlerCrawler::new(
                    target,
                    settings,
                    controller_sender,
                    console_sender,
                    scoped_domains,
                );
                let result = crawler.crawl(v);
                if let Ok(_result) = result {
                    crawler.finish()
                }
            });
            controller_receivers.push(controller_receiver);
        }
    }

    async fn start_console(
        console_receiver: Receiver<ConsoleMessage>,
        thread_pool: &ThreadPool,
        settings: RinzlerSettings,
    ) -> Result<(), Box<dyn Error>> {
        let console = RinzlerConsole::new(settings.clone(), console_receiver)?;
        thread_pool.execute(move || {
            console
                .clear()
                .banner(format!("{}", settings.clone()))
                .render();
        });
        Ok(())
    }
}
