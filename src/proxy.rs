use clap_complete::shells::Shell;

pub fn proxy_export_cmd(hostname: &str, http_port: &u16, socks_port: &u16) -> String {
    // Check current shell
    let shell = Shell::from_env().unwrap_or(Shell::Bash);
    match shell {
        Shell::Fish => {
            // For fish, use `set -gx $ENV_VAR value` to set environment variables
            format!(
                "set -gx https_proxy http://{hostname}:{http_port} \
                set -gx http_proxy http://{hostname}:{http_port} \
                set -gx all_proxy socks5://{hostname}:{socks_port}"
            )
        }
        _ => {
            // For all other shells (bash/zsh), use `export $ENV_VAR=value`
            format!(
                "export https_proxy=http://{hostname}:{http_port} \
                http_proxy=http://{hostname}:{http_port} \
                all_proxy=socks5://{hostname}:{socks_port}"
            )
        }
    }
}

pub fn proxy_unset_cmd() -> String {
    // Check current shell
    let shell = Shell::from_env().unwrap_or(Shell::Bash);
    match shell {
        Shell::Fish => {
            // For fish, use `set -e $ENV_VAR` to unset environment variables
            "set -e https_proxy http_proxy all_proxy".to_owned()
        }
        _ => {
            // For all other shells (bash/zsh), use `unset $ENV_VAR`
            "unset https_proxy http_proxy all_proxy".to_owned()
        }
    }
}
