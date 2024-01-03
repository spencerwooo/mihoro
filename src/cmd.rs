use clap::Parser;
use clap::Subcommand;

#[derive(Parser)]
#[command(author, about, version, arg_required_else_help(true))]
pub struct Args {
    /// Path to mihoro config file
    #[clap(short, long, default_value = "~/.config/mihoro.toml")]
    pub mihoro_config: String,
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Setup mihoro by downloading mihomo binary and remote config")]
    Setup,
    #[command(about = "Update mihomo remote config and restart mihomo.service")]
    Update,
    #[command(about = "Apply mihomo config overrides and restart mihomo.service")]
    Apply,
    #[command(about = "Start mihomo.service with systemctl")]
    Start,
    #[command(about = "Check mihomo.service status with systemctl")]
    Status,
    #[command(about = "Stop mihomo.service with systemctl")]
    Stop,
    #[command(about = "Restart mihomo.service with systemctl")]
    Restart,
    #[command(about = "Check mihomo.service logs with journalctl")]
    Log,
    #[command(about = "Output proxy export commands")]
    Proxy {
        #[clap(subcommand)]
        proxy: Option<ProxyCommands>,
    },
    #[command(about = "Uninstall and remove mihoro and config")]
    Uninstall,
    #[command(about = "Generate shell completions for mihoro")]
    Completions {
        #[clap(subcommand)]
        shell: Option<ClapShell>,
    },
}

#[derive(Subcommand)]
#[command(arg_required_else_help(true))]
pub enum ProxyCommands {
    #[command(about = "Output and copy proxy export shell commands")]
    Export,
    #[command(about = "Output and copy proxy export shell commands for LAN access")]
    ExportLan,
    #[command(about = "Output and copy proxy unset shell commands")]
    Unset,
}

#[derive(Subcommand)]
#[command(arg_required_else_help(true))]
pub enum ClapShell {
    #[command(about = "Generate bash completions")]
    Bash,
    #[command(about = "Generate fish completions")]
    Fish,
    #[command(about = "Generate zsh completions")]
    Zsh,
    // #[command(about = "Generate powershell completions")]
    // Powershell,
    // #[command(about = "Generate elvish completions")]
    // Elvish,
}
