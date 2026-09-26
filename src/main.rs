mod api;
mod app;
mod config;
mod model;
mod theme;
mod ui;
mod worker;

use anyhow::Result;
use clap::{Parser, Subcommand};
use crossterm::event::{Event as TermEvent, EventStream, KeyEventKind};
use futures::StreamExt;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "zabterm", version, about = "A beautiful terminal UI for Zabbix")]
struct Cli {
    /// Profile from the config file to use.
    #[arg(short, long, global = true)]
    profile: Option<String>,

    /// Path to an alternative config file.
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,

    /// Override the theme (auto, tokyo-night, catppuccin, gruvbox, ...).
    #[arg(short, long)]
    theme: Option<String>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Write a starter config to ~/.config/zabterm/config.toml.
    Init {
        #[arg(long)]
        force: bool,
    },
    /// Verify that the active profile can reach and authenticate to Zabbix.
    Check,
    /// Print the config file path.
    Config,
}

#[tokio::main]
async fn main() {
    if let Err(e) = run(Cli::parse()).await {
        eprintln!("\x1b[31m✕\x1b[0m {e:#}");
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<()> {
    let config_path = cli.config.clone().unwrap_or_else(config::config_path);
    match cli.command {
        Some(Command::Init { force }) => {
            let path = config::write_template(force)?;
            println!("\x1b[32m✓\x1b[0m wrote {}", path.display());
            println!("  add your Zabbix URL and API token, then run \x1b[1mzabterm\x1b[0m");
            return Ok(());
        }
        Some(Command::Config) => {
            println!("{}", config_path.display());
            return Ok(());
        }
        Some(Command::Check) => return check(&cli, &config_path).await,
        None => {}
    }

    let mut config = config::Config::load(cli.config.as_ref())?;
    if let Some(theme) = cli.theme {
        config.theme = theme;
    }
    let profile = config.profile(cli.profile.as_deref())?;
    // Fail fast on missing credentials instead of inside the TUI.
    profile.resolve_auth()?;
    tui(config, profile, config_path).await
}

async fn check(cli: &Cli, config_path: &std::path::Path) -> Result<()> {
    let config = config::Config::load(cli.config.as_ref())?;
    let profile = config.profile(cli.profile.as_deref())?;
    println!("  config   {}", config_path.display());
    println!("  profile  {}", profile.name);
    println!("  endpoint {}", profile.api_url());
    let client = api::Client::connect(&profile).await?;
    let version = client.version().await?;
    let hosts = client.hosts().await?;
    let problems = client.problems().await?;
    println!("\x1b[32m✓\x1b[0m Zabbix {version} · {} hosts · {} active problems", hosts.len(), problems.len());
    Ok(())
}

async fn tui(config: config::Config, profile: config::Profile, config_path: PathBuf) -> Result<()> {
    let (ev_tx, mut ev_rx) = tokio::sync::mpsc::unbounded_channel();
    let cmd_tx = worker::spawn(profile.clone(), config.refresh_interval, ev_tx);
    let mut app = app::App::new(config, profile, config_path, cmd_tx);

    let mut terminal = ratatui::init();
    let mut keys = EventStream::new();
    let mut tick = tokio::time::interval(Duration::from_millis(250));

    let result: Result<()> = async {
        loop {
            terminal.draw(|f| ui::draw(f, &mut app))?;
            tokio::select! {
                _ = tick.tick() => app.on_tick(),
                Some(ev) = ev_rx.recv() => app.on_event(ev),
                Some(Ok(ev)) = keys.next() => {
                    if let TermEvent::Key(key) = ev
                        && key.kind == KeyEventKind::Press {
                            app.on_key(key);
                        }
                }
            }
            if app.quit {
                return Ok(());
            }
        }
    }
    .await;

    ratatui::restore();
    result
}
