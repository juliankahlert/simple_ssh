use anyhow::Result;
use simple_ssh::Session;

mod cli;
use cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::with("host")?.and("user")?.and("key")?;

    let mut ssh = Session::init()
        .with_host(&args.host.expect("missing host from CLI"))
        .with_user(&args.user.expect("missing user from CLI"))
        .with_key(args.key.expect("missing key from CLI").into())
        .with_port(args.port.unwrap_or(22))
        .build()?
        .connect()
        .await?;

    ssh.scp("Cargo.toml", "/tmp/Cargo.toml").await?;
    println!("File uploaded successfully");

    ssh.close().await?;
    Ok(())
}

#[cfg(test)]
mod example_tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_scp_example_compiles() {
        let cli = Cli::parse_from(["scp", "--host", "example.com"]);
        assert_eq!(cli.host, Some("example.com".to_string()));
    }

    #[test]
    fn test_scp_example_with_key() {
        let cli = Cli::parse_from([
            "scp",
            "--host", "example.com",
            "--user", "admin",
            "--key", "/path/to/key"
        ]);
        assert!(cli.host.is_some());
        assert!(cli.user.is_some());
        assert!(cli.key.is_some());
    }

    #[test]
    fn test_scp_example_with_port() {
        let cli = Cli::parse_from([
            "scp",
            "--host", "example.com",
            "--port", "3333"
        ]);
        assert_eq!(cli.port, Some(3333));
    }
}
