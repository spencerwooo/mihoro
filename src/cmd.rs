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
    /// Initialize mihoro: download binary, config, geodata, and set up the systemd service
    Init {
        /// Re-download all artifacts even if they already exist
        #[arg(long)]
        force: bool,

        /// Non-interactive mode: fail if required config fields are missing instead of prompting
        #[arg(short = 'y', long)]
        yes: bool,

        /// Override architecture detection
        ///
        /// Supported options on Linux: 386, 386-go120, 386-go123, 386-softfloat, amd64,
        /// amd64-compatible, amd64-v1/v2/v3 (with -go120/-go123 variants),
        /// arm64, armv5, armv6, armv7, loong64-abi1/abi2, mips-hardfloat,
        /// mips-softfloat, mips64, mips64le, mipsle-hardfloat, mipsle-softfloat,
        /// ppc64le, riscv64, s390x
        #[arg(long)]
        arch: Option<String>,
    },
    /// Deprecated: use `mihoro init` instead
    #[command(hide = true)]
    Setup {
        /// Force download mihomo binary even if it already exists
        #[arg(long)]
        overwrite: bool,

        /// Override architecture detection
        #[arg(long)]
        arch: Option<String>,
    },
    /// Update mihomo components (config by default)
    Update {
        /// Update remote config
        #[arg(long)]
        config: bool,

        /// Update external UI assets
        #[arg(long)]
        ui: bool,

        /// Update mihomo core binary
        #[arg(long)]
        core: bool,

        /// Update geodata
        #[arg(long)]
        geodata: bool,

        /// Update everything: config, geodata, and mihomo core binary
        #[arg(long, conflicts_with_all = ["config", "ui", "core", "geodata"])]
        all: bool,

        /// Override architecture detection (used with --core or --all)
        ///
        /// Supported options on Linux: 386, 386-go120, 386-go123, 386-softfloat, amd64,
        /// amd64-compatible, amd64-v1/v2/v3 (with -go120/-go123 variants),
        /// arm64, armv5, armv6, armv7, loong64-abi1/abi2, mips-hardfloat,
        /// mips-softfloat, mips64, mips64le, mipsle-hardfloat, mipsle-softfloat,
        /// ppc64le, riscv64, s390x
        #[arg(long)]
        arch: Option<String>,
    },
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
    /// Manage auto-update cron job
    Cron {
        #[clap(subcommand)]
        cron: Option<CronCommands>,
    },
    #[cfg_attr(not(feature = "self_update"), command(hide = true))]
    /// Upgrade mihoro to the latest version
    Upgrade {
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,

        /// Only check for updates, don't install
        #[arg(long)]
        check: bool,

        /// Override target triple (e.g., x86_64-unknown-linux-gnu)
        #[arg(long)]
        target: Option<String>,
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

#[derive(Subcommand)]
#[command(arg_required_else_help(true))]
pub enum CronCommands {
    /// Enable auto-update cron job
    Enable,
    /// Disable auto-update cron job
    Disable,
    /// Show auto-update cron job status
    Status,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_update_ui_flag() {
        let args = Args::parse_from(["mihoro", "update", "--ui"]);
        match args.command {
            Some(Commands::Update {
                ui,
                config,
                core,
                geodata,
                all,
                ..
            }) => {
                assert!(ui);
                assert!(!config);
                assert!(!core);
                assert!(!geodata);
                assert!(!all);
            }
            _ => panic!("expected update command"),
        }
    }

    #[test]
    fn test_parse_update_all_flag() {
        let args = Args::parse_from(["mihoro", "update", "--all"]);
        match args.command {
            Some(Commands::Update { all, ui, .. }) => {
                assert!(all);
                assert!(!ui);
            }
            _ => panic!("expected update command"),
        }
    }
}
