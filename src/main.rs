mod cmd;
mod config;
mod mihoro;
mod proxy;
mod systemctl;
mod utils;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete::{
    generate,
    shells::{Bash, Fish, Zsh},
};
use colored::Colorize;
use reqwest::Client;
use std::{io, process::Command};

use cmd::{Args, ClapShell, Commands};
use mihoro::Mihoro;
use systemctl::Systemctl;

#[tokio::main]
async fn main() {
    if let Err(err) = cli().await {
        eprintln!("{} {}", "error:".bright_red().bold(), err);
        std::process::exit(1);
    }
}

async fn cli() -> Result<()> {
    let args = Args::parse();
    let client = Client::new();
    let mihoro = Mihoro::new(&args.mihoro_config)?;

    match &args.command {
        Some(Commands::Setup) => mihoro.setup(client).await?,
        Some(Commands::Update) => mihoro.update(client).await?,
        Some(Commands::UpdateGeodata) => mihoro.update_geodata(client).await?,
        Some(Commands::Apply) => mihoro.apply().await?,
        Some(Commands::Uninstall) => mihoro.uninstall()?,
        Some(Commands::Proxy { proxy }) => mihoro.proxy_commands(proxy)?,

        Some(Commands::Start) => Systemctl::new()
            .start("mihomo.service")
            .execute()
            .map(|_| {
                println!("{} Started mihomo.service", mihoro.prefix.green());
            })?,

        Some(Commands::Status) => {
            Systemctl::new().status("mihomo.service").execute()?;
        }

        Some(Commands::Stop) => Systemctl::new().stop("mihomo.service").execute().map(|_| {
            println!("{} Stopped mihomo.service", mihoro.prefix.green());
        })?,

        Some(Commands::Restart) => {
            Systemctl::new()
                .restart("mihomo.service")
                .execute()
                .map(|_| {
                    println!("{} Restarted mihomo.service", mihoro.prefix.green());
                })?
        }

        Some(Commands::Log) => {
            Command::new("journalctl")
                .arg("--user")
                .arg("-xeu")
                .arg("mihomo.service")
                .arg("-n")
                .arg("10")
                .arg("-f")
                .spawn()
                .expect("failed to execute process")
                .wait()?;
        }

        Some(Commands::Completions { shell }) => match shell {
            Some(ClapShell::Bash) => {
                generate(Bash, &mut Args::command(), "mihoro", &mut io::stdout())
            }
            Some(ClapShell::Zsh) => {
                generate(Zsh, &mut Args::command(), "mihoro", &mut io::stdout())
            }
            Some(ClapShell::Fish) => {
                generate(Fish, &mut Args::command(), "mihoro", &mut io::stdout())
            }
            _ => (),
        },

        None => (),
    }
    Ok(())
}
