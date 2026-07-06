use std::time::{Duration, Instant};

use anyhow::Result;
use clap::Parser;
use tracing::{error, info};
use vico_desktop_client::types::ContextMessage;

mod app;
mod history;
mod theme;
mod types;
mod ui;
mod vico;

use crate::vico::VicoClient;

#[derive(Parser, Debug)]
#[command(name = "vicount")]
#[command(about = "Vicount — a Rust TUI for ViCo Desktop")]
struct Cli {
    /// Non-interactive prompt mode: send a prompt and print the response.
    #[arg(short, long)]
    prompt: Option<String>,

    /// Force the TUI even when no prompt is provided (default behavior).
    #[arg(long)]
    tui: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    if let Some(prompt) = cli.prompt {
        return run_non_interactive(&prompt).await;
    }

    if cli.tui || cli.prompt.is_none() {
        return app::run_app().await;
    }

    Ok(())
}

async fn run_non_interactive(prompt: &str) -> Result<()> {
    info!("non-interactive mode: {prompt}");
    let vico = VicoClient::new();

    let url = vico.url();
    if url == "offline" {
        eprintln!("(offline)");
    } else {
        eprintln!("(ViCo URL: {url})");
    }

    let _started = Instant::now();
    let spinner = tokio::spawn(async move {
        let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let mut i = 0;
        loop {
            tokio::time::sleep(Duration::from_millis(80)).await;
            eprint!("\r{} thinking...", frames[i % frames.len()]);
            i += 1;
        }
    });

    let context = vec![ContextMessage {
        role: "user".to_string(),
        content: prompt.to_string(),
        agent: None,
    }];

    let result = match vico.chat(prompt, context).await {
        Ok(text) => text,
        Err(e) => {
            error!("chat failed: {e}");
            format!("error: {e}")
        }
    };

    spinner.abort();
    eprintln!();
    println!("{}", result);
    Ok(())
}
