use app::RinzlerApplication;
use config::parse_cmd_line;

mod app;
mod config;
mod crawler;
mod ui;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = parse_cmd_line();
    let app = RinzlerApplication::from_settings(settings);
    app.run().await?;
    Ok(())
}
