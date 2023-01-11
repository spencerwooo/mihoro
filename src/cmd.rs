use clap::Parser;
use clap::Subcommand;

#[derive(Parser)]
#[command(author, about, version)]
pub struct Args {
    /// Path to clashrup config file
    #[clap(short, long, default_value = "~/.config/clashrup.toml")]
    pub clashrup_config: String,
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Setup clashrup by downloading clash binary and remote config")]
    Setup,
    #[command(about = "Update clash remote config, mmdb, and restart clash.service")]
    Update,
    #[command(about = "Apply clash config overrides and restart clash.service")]
    Apply,
    #[command(about = "Start clash.service with systemctl")]
    Start,
    #[command(about = "Check clash.service status with systemctl")]
    Status,
    #[command(about = "Stop clash.service with systemctl")]
    Stop,
    #[command(about = "Restart clash.service with systemctl")]
    Restart,
    #[command(about = "Check clash.service logs with journalctl")]
    Log,
    #[command(about = "Proxy export commands, `clashrup proxy --help` to see more")]
    Proxy {
        #[command(subcommand)]
        proxy: Option<ProxyCommands>,
    },
    #[command(about = "Uninstall and remove clash and config")]
    Uninstall,
}

#[derive(Subcommand)]
pub enum ProxyCommands {
    #[command(about = "Output and copy proxy export shell commands")]
    Export,
    #[command(about = "Output and copy proxy export shell commands for LAN access")]
    ExportLan,
    #[command(about = "Output and copy proxy unset shell commands")]
    Unset,
}
