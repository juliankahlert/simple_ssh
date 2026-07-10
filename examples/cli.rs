use anyhow::{anyhow, Result};
use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[arg(long)]
    pub host: Option<String>,

    #[arg(long)]
    pub user: Option<String>,

    #[arg(long)]
    pub passwd: Option<String>,

    #[arg(long)]
    pub port: Option<u16>,

    #[arg(long)]
    pub key: Option<String>,

    #[arg(long)]
    pub command: Option<String>,

    #[arg(long)]
    pub timeout: Option<u64>,

    #[arg(long)]
    pub scope: Option<String>,
}

impl Cli {
    pub fn with(field: &str) -> Result<Self> {
        let cli = Cli::parse();
        cli.and(field)
    }

    pub fn and(mut self, field: &str) -> Result<Self> {
        match field {
            "host" => {
                if self.host.is_none() {
                    self.host = Some(prompt("host")?);
                }
            }
            "user" => {
                if self.user.is_none() {
                    self.user = Some(prompt("user")?);
                }
            }
            "passwd" => {
                if self.passwd.is_none() {
                    self.passwd = Some(prompt("passwd")?);
                }
            }
            "port" => {
                if self.port.is_none() {
                    self.port = Some(prompt("port")?.parse()?);
                }
            }
            "key" => {
                if self.key.is_none() {
                    self.key = Some(prompt("key")?);
                }
            }
            "command" => {
                if self.command.is_none() {
                    self.command = Some(prompt("command")?);
                }
            }
            "timeout" => {
                if self.timeout.is_none() {
                    self.timeout = Some(prompt("timeout")?.parse()?);
                }
            }
            "scope" => {
                if self.scope.is_none() {
                    self.scope = Some(prompt("scope")?);
                }
            }
            _ => return Err(anyhow!("unknown field: {}", field)),
        }
        Ok(self)
    }
}

fn prompt(field: &str) -> Result<String> {
    use std::io::{self, Write};
    print!("Enter {}: ", field);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_with_args_host() {
        let cli = Cli::parse_from(["cli", "--host", "example.com"]);
        assert!(cli.host.is_some());
        assert_eq!(cli.host.unwrap(), "example.com");
    }

    #[test]
    fn test_cli_with_args_user() {
        let cli = Cli::parse_from(["cli", "--host", "example.com", "--user", "testuser"]);
        assert!(cli.host.is_some());
        assert!(cli.user.is_some());
        assert_eq!(cli.user.unwrap(), "testuser");
    }

    #[test]
    fn test_cli_and_unknown_field() {
        let cli = Cli::parse_from(["cli", "--host", "test.com"]);
        let result = cli.and("unknown");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "unknown field: unknown");
    }

    #[test]
    fn test_cli_empty_parsing() {
        let cli = Cli::parse_from(["cli"]);
        assert!(cli.host.is_none());
        assert!(cli.user.is_none());
        assert!(cli.passwd.is_none());
        assert!(cli.port.is_none());
        assert!(cli.key.is_none());
        assert!(cli.command.is_none());
        assert!(cli.timeout.is_none());
        assert!(cli.scope.is_none());
    }

    #[test]
    fn test_cli_with_args_scope() {
        let cli = Cli::parse_from(["cli", "--host", "example.com", "--scope", "all"]);
        assert!(cli.host.is_some());
        assert!(cli.scope.is_some());
        assert_eq!(cli.scope.unwrap(), "all");
    }

    #[test]
    fn test_cli_with_args_all_fields() {
        let cli = Cli::parse_from([
            "cli",
            "--host",
            "example.com",
            "--user",
            "testuser",
            "--passwd",
            "password",
        ]);
        assert!(cli.host.is_some());
        assert!(cli.user.is_some());
        assert!(cli.passwd.is_some());
    }

    #[test]
    fn test_cli_from_args_exact() {
        let cli = Cli::parse_from([
            "cli",
            "--host",
            "example.com",
            "--user",
            "testuser",
            "--passwd",
            "password",
        ]);
        assert!(cli.host.is_some());
        assert!(cli.user.is_some());
        assert!(cli.passwd.is_some());
    }

    #[test]
    fn test_cli_from_args_duplicate_field() {
        let cli = Cli::parse_from(["cli", "--host", "example.com", "--host", "example2.com"]);
        assert_eq!(cli.host, Some("example2.com".to_string()));
    }

    #[test]
    fn test_cli_from_command_line_args() {
        let cli = Cli::parse_from([
            "cli",
            "--host",
            "test.example.com",
            "--user",
            "testuser",
            "--port",
            "22",
        ]);
        assert_eq!(cli.host, Some("test.example.com".to_string()));
        assert_eq!(cli.user, Some("testuser".to_string()));
        assert_eq!(cli.port, Some(22));
    }
}

#[allow(dead_code)]
fn main() {
    let cli = Cli::parse();
    println!("{:?}", cli);
}
