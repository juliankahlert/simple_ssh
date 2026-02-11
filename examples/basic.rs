use anyhow::Result;
use simple_ssh::Session;

mod cli;
use cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::with("host")?.and("user")?.and("passwd")?;

    let mut ssh = Session::init()
        .with_host(&args.host.expect("host must be provided"))
        .with_user(&args.user.expect("user must be provided"))
        .with_passwd(&args.passwd.expect("passwd must be provided"))
        .with_port(args.port.unwrap_or(22))
        .build()?
        .connect()
        .await?;

    let code = ssh.cmd("uname -a").await?;
    println!("Exit code: {}", code);

    ssh.close().await?;
    Ok(())
}

#[cfg(test)]
mod example_tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_basic_example_compiles() {
        let cli = Cli::parse_from(["basic", "--host", "example.com"]);
        assert_eq!(cli.host, Some("example.com".to_string()));
    }

    #[test]
    fn test_basic_example_with_port() {
        let cli = Cli::parse_from(["basic", "--host", "example.com", "--port", "2222"]);
        assert_eq!(cli.host, Some("example.com".to_string()));
        assert_eq!(cli.port, Some(2222));
    }

    #[test]
    fn test_basic_example_with_all_auth() {
        let cli = Cli::parse_from([
            "basic",
            "--host",
            "example.com",
            "--user",
            "admin",
            "--passwd",
            "secret",
        ]);
        assert!(cli.host.is_some());
        assert!(cli.user.is_some());
        assert!(cli.passwd.is_some());
    }
}
