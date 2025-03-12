use clap::{Parser, Subcommand};

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
    /// Setup mihoro by downloading mihomo binary and remote config
    Setup {
        /// Force download mihomo binary even if it already exists
        #[arg(long)]
        overwrite: bool,
    },
    /// Update mihomo remote config and restart mihomo.service
    Update,
    /// Update mihomo geodata
    UpdateGeodata,
    /// Apply mihomo config overrides and restart mihomo.service
    Apply,
    /// Start mihomo.service with systemctl
    Start,
    /// Check mihomo.service status with systemctl
    Status,
    /// Stop mihomo.service with systemctl
    Stop,
    /// Restart mihomo.service with systemctl
    Restart,
    /// Check mihomo.service logs with journalctl
    #[clap(visible_alias("logs"))]
    Log,
    /// Output proxy export commands
    Proxy {
        #[clap(subcommand)]
        proxy: Option<ProxyCommands>,
    },
    /// Uninstall and remove mihoro and config
    Uninstall,
    /// Generate shell completions for mihoro
    Completions {
        #[clap(subcommand)]
        shell: Option<ClapShell>,
    },
}

#[derive(Subcommand)]
#[command(arg_required_else_help(true))]
pub enum ProxyCommands {
    /// Output and copy proxy export shell commands
    Export,
    /// Output and copy proxy export shell commands for LAN access
    ExportLan,
    /// Output and copy proxy unset shell commands
    Unset,
}

#[derive(Subcommand)]
#[command(arg_required_else_help(true))]
pub enum ClapShell {
    /// Generate bash completions
    Bash,
    /// Generate fish completions
    Fish,
    /// Generate zsh completions
    Zsh,
    // #[command(about = "Generate powershell completions")]
    // Powershell,
    // #[command(about = "Generate elvish completions")]
    // Elvish,
}
