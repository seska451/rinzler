use crate::config::RinzlerSettings;
use crate::crawler::crawl_target::CrawlTarget;
use colored::Colorize;
use console::{Emoji, Term};
use crossbeam::channel::{unbounded, Receiver};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::thread::sleep;
use std::time::Duration;
use std::{io, thread};

static SPIDER_WEB: Emoji = Emoji("🕸️", "|");
static HEARTS: Emoji = Emoji("💖💖💖", "<3 ");
static GREEN_CHECK: Emoji = Emoji("  ✅  ", ":");
static CROSS_MARK: Emoji = Emoji("  ❌  ", ":");

pub enum ConsoleMessageType {
    ForceBrowseStart,
    ForceBrowseProgress,
    ForceBrowseHit,
    ForceBrowseAttempt,
    Finish,
    Abort,
    Result,
    NONE,
}

pub struct ConsoleMessage {
    pub(crate) message_type: ConsoleMessageType,
    pub(crate) data: Result<String, String>,
    pub(crate) original_target: Option<CrawlTarget>,
    pub(crate) crawl_target: Option<CrawlTarget>,
    pub(crate) total: Option<u64>,
}
impl Clone for ConsoleMessage {
    fn clone(&self) -> Self {
        ConsoleMessage {
            message_type: ConsoleMessageType::NONE,
            data: self.data.clone(),
            original_target: self.original_target.clone(),
            crawl_target: self.crawl_target.clone(),
            total: self.total.clone(),
        }
    }
}

pub(crate) struct RinzlerConsole {
    settings: RinzlerSettings,
    message_receiver: Receiver<ConsoleMessage>,
    terminal: Term,
}

impl RinzlerConsole {
    pub fn new(
        settings: RinzlerSettings,
        message_receiver: Receiver<ConsoleMessage>,
    ) -> Result<RinzlerConsole, io::Error> {
        Ok(RinzlerConsole {
            settings,
            message_receiver,
            terminal: Term::stdout(),
        })
    }

    pub fn clear(self) -> RinzlerConsole {
        let _ = self.terminal.clear_screen();
        self
    }

    fn spawn_stdin_channel() -> Receiver<String> {
        let (tx, rx) = unbounded();
        thread::spawn(move || loop {
            let mut buffer = String::new();
            io::stdin().read_line(&mut buffer).unwrap();
            tx.send(buffer).unwrap();
        });
        rx
    }

    pub fn render(self) {
        let m = MultiProgress::new();
        let mut ongoing_scans: HashMap<CrawlTarget, ProgressBar> = HashMap::new();
        let stdin_channel = RinzlerConsole::spawn_stdin_channel();
        loop {
            if let Ok(key) = stdin_channel.try_recv() {
                if key == "\n" {
                    break;
                }
            }
            sleep(Duration::from_millis(50));
            let console_message = self.message_receiver.try_recv();
            if let Ok(command) = console_message {
                match command.message_type {
                    ConsoleMessageType::NONE => {}
                    ConsoleMessageType::ForceBrowseStart => {
                        let pb = m.add(ProgressBar::new(command.total.unwrap()));
                        pb.set_style(ProgressStyle::default_bar()
                            .template("{spinner:.green} {msg:50}\n[{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos:>7}/{len:7} ({eta}) ")
                            .with_key("eta", |state| format!("{:.1}s", state.eta().as_secs_f64()))
                            .progress_chars("#>-"));
                        ongoing_scans.insert(command.crawl_target.unwrap(), pb);
                    }
                    ConsoleMessageType::ForceBrowseProgress => {
                        let pb = &ongoing_scans.get(&command.crawl_target.unwrap()).unwrap();
                        pb.inc(1);
                    }
                    ConsoleMessageType::ForceBrowseHit => {
                        let ct = &command.crawl_target.clone();
                        let pb = &ongoing_scans.get(&ct.clone().unwrap()).unwrap();
                        pb.println(format!("{}", &ct.clone().unwrap()));
                        pb.inc(1);
                    }
                    ConsoleMessageType::ForceBrowseAttempt => {
                        let c3 = command.clone();
                        let old = c3.original_target.unwrap();
                        let pb = &ongoing_scans.get(&old).unwrap();
                        let new = c3.crawl_target.unwrap();
                        pb.set_message(format!("{}", new.url));
                    }
                    ConsoleMessageType::Finish => {
                        let output = format!(
                            "\n{} Scan Finished: {}\n",
                            GREEN_CHECK,
                            &command.data.unwrap().as_str().green()
                        );

                        let _ = self.terminal.write_line(output.as_str());
                        break;
                    }
                    ConsoleMessageType::Abort => {
                        if let Err(error) = command.data {
                            let output = format!("\n{} Scan Failed: {}\n", CROSS_MARK, error.red());

                            let _ = self.terminal.write_line(output.as_str());
                        };
                        break;
                    }
                    ConsoleMessageType::Result => {
                        let _ = if !self.settings.quiet {
                            if let Some(crawl_tgt) = command.crawl_target {
                                if HashMap::contains_key(&ongoing_scans, &crawl_tgt) {
                                    let pb = ongoing_scans.remove(&crawl_tgt).unwrap();
                                    if crawl_tgt.status_code.is_some() {
                                        pb.finish_with_message(format!("{}", crawl_tgt));
                                    }
                                } else {
                                    if crawl_tgt.status_code.is_none() {
                                        let pb = m.add(Self::get_spinner(&crawl_tgt));
                                        ongoing_scans.insert(crawl_tgt, pb);
                                    } else {
                                        println!("{}", crawl_tgt);
                                    }
                                }
                            }
                            for n in &ongoing_scans {
                                n.1.inc(1);
                            }
                        };
                    }
                }
            }
        }
    }

    fn get_spinner(crawl_tgt: &CrawlTarget) -> ProgressBar {
        let pb = ProgressBar::new_spinner().with_message(format!("{}", crawl_tgt));
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_chars(RinzlerConsole::get_spinner_chars())
                .template("{prefix:.bold.dim} {spinner} {wide_msg}"),
        );
        pb
    }

    fn get_spinner_chars() -> &'static str {
        "⠁⠂⠄⡀⢀⠠⠐⠈✓"
    }

    pub fn banner(self, settings_desc: String) -> RinzlerConsole {
        let mut builder = string_builder::Builder::default();

        builder.append("           _             __\n");
        builder.append("     _____(_)___  ____  / /__  _____\n");
        builder.append("    / ___/ / __ \\/_  / / / _ \\/ ___/\n");
        builder.append("   / /  / / / / / / /_/ /  __/ /\n");
        builder.append("  /_/  /_/_/ /_/ /___/_/\\___/_/\n");
        builder.append(format!("  v{}\n\n", env!("CARGO_PKG_VERSION")));
        builder.append(format!("  {}    a fast webcrawler\n", SPIDER_WEB));
        builder.append(format!("  {}    from seska with {}\n", SPIDER_WEB, HEARTS));
        builder.append(format!("  {}\n", SPIDER_WEB));
        builder.append(format!("  {}    usage: rnz <URL>\n", SPIDER_WEB));
        builder.append(format!("  {}\n", SPIDER_WEB));
        builder.append(format!("  {}    Press 'enter' to quit\n\n", SPIDER_WEB));
        builder.append(format!("{}\n", settings_desc));

        print!("{}", builder.string().unwrap());
        self
    }
}
