use std::collections::{HashMap};
use std::{io};
use std::sync::mpsc::Receiver;
use std::time::Duration;
use colored::Colorize;
use console::{Emoji, Term};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use crate::{CrawlTarget, Settings};

static SPIDER_WEB: Emoji = Emoji("🕸️", "|");
static HEARTS: Emoji = Emoji("💖💖💖", "<3 ");
static GREEN_CHECK: Emoji = Emoji("  ✅  ", ":");
static CROSS_MARK: Emoji = Emoji("  ❌  ", ":");

pub enum ConsoleMessageType {
    FINISH,
    ABORT,
    RESULT,
}

pub struct ConsoleMessage {
    pub(crate) message_type: ConsoleMessageType,
    pub(crate) data: Result<String, String>,
    pub(crate) crawl_target: Option<CrawlTarget>
}

pub(crate) struct RinzlerConsole {
    settings: Settings,
    message_receiver: Receiver<ConsoleMessage>,
    terminal: Term,
}

impl RinzlerConsole {
    pub fn new(settings: Settings, message_receiver: Receiver<ConsoleMessage>) -> Result<RinzlerConsole, io::Error> {
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

    pub fn render(self) {
        let m = MultiProgress::new();
        let mut ongoing_scans: HashMap<CrawlTarget, ProgressBar> = HashMap::new();
        loop {
            let console_message = self.message_receiver.try_recv();
            if let Ok(command) = console_message {
                match command.message_type {
                    ConsoleMessageType::FINISH => {
                        let output = format!(
                            "\n{} Scan Finished: {}\n",
                            GREEN_CHECK,
                            &command.data.unwrap().as_str().green());

                        let _ = self.terminal.write_line(output.as_str());
                        break;
                    },
                    ConsoleMessageType::ABORT => {
                        if let Err(error) = command.data {
                            let output = format!(
                                "\n{} Scan Failed: {}\n",
                                CROSS_MARK,
                                error.red());

                            let _ = self.terminal.write_line(output.as_str());
                        };
                        break;
                    },
                    ConsoleMessageType::RESULT => {
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
                            std::thread::sleep(Duration::from_millis(10));
                        };
                    }
                }
            }
        }
    }

    fn get_spinner(crawl_tgt: &CrawlTarget) -> ProgressBar {
        let pb = ProgressBar::new_spinner()
            .with_message(format!("{}", crawl_tgt));
        pb.set_style(ProgressStyle::default_spinner()
            .tick_chars(RinzlerConsole::get_spinner_chars())
            .template("{prefix:.bold.dim} {spinner} {wide_msg}"));
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
        builder.append(format!("  {}    Press 'q' to quit\n\n", SPIDER_WEB));
        builder.append(format!("{}\n", settings_desc));

        print!("{}", builder.string().unwrap());
        self
    }
}