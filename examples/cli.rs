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
}

impl Cli {
    pub fn with(field: &str) -> Result<Self> {
        let cli = Cli::parse();
        cli.and(field)
    }

    pub fn and(mut self, field: &str) -> Result<Self> {
        match field {
            "host" => {
                if self.host.is_some() {
                    return Err(anyhow!("host already set"));
                }
                self.host = Some(prompt("host")?);
            }
            "user" => {
                if self.user.is_some() {
                    return Err(anyhow!("user already set"));
                }
                self.user = Some(prompt("user")?);
            }
            "passwd" => {
                if self.passwd.is_some() {
                    return Err(anyhow!("passwd already set"));
                }
                self.passwd = Some(prompt("passwd")?);
            }
            "port" => {
                if self.port.is_some() {
                    return Err(anyhow!("port already set"));
                }
                self.port = Some(prompt("port")?.parse()?);
            }
            "key" => {
                if self.key.is_some() {
                    return Err(anyhow!("key already set"));
                }
                self.key = Some(prompt("key")?);
            }
            "command" => {
                if self.command.is_some() {
                    return Err(anyhow!("command already set"));
                }
                self.command = Some(prompt("command")?);
            }
            "timeout" => {
                if self.timeout.is_some() {
                    return Err(anyhow!("timeout already set"));
                }
                self.timeout = Some(prompt("timeout")?.parse()?);
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
    fn test_cli_with_host() {
        let cli = Cli::with("host").unwrap();
        assert!(cli.host.is_some());
    }

    #[test]
    fn test_cli_and_user() {
        let cli = Cli::with("host").unwrap().and("user").unwrap();
        assert!(cli.host.is_some());
        assert!(cli.user.is_some());
    }

    #[test]
    fn test_cli_and_unknown_field() {
        let result = Cli::with("host").unwrap().and("unknown");
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
    }
}

#[allow(dead_code)]
fn main() {
    let cli = Cli::parse();
    println!("{:?}", cli);
}
