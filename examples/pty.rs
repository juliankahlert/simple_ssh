use anyhow::Result;
use simple_ssh::Session;

mod cli;
use cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::with("host")?.and("user")?.and("key")?;

    let mut ssh = Session::init()
        .with_host(&args.host.expect("missing host"))
        .with_user(&args.user.expect("missing user"))
        .with_key(args.key.expect("missing key").into())
        .with_port(args.port.unwrap_or(22))
        .build()?
        .connect()
        .await?;

    let exit_code = ssh
        .pty_builder()
        .with_raw()
        .with_term("xterm-256color")
        .with_auto_resize()
        .run()
        .await?;

    println!("Exit code: {}", exit_code);

    ssh.close().await?;
    Ok(())
}

#[cfg(test)]
mod example_tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_pty_example_compiles() {
        let cli = Cli::parse_from(["pty", "--host", "example.com"]);
        assert_eq!(cli.host, Some("example.com".to_string()));
    }

    #[test]
    fn test_pty_example_with_key() {
        let cli = Cli::parse_from([
            "pty",
            "--host",
            "example.com",
            "--user",
            "admin",
            "--key",
            "/path/to/key",
        ]);
        assert!(cli.host.is_some());
        assert!(cli.user.is_some());
        assert!(cli.key.is_some());
    }

    #[test]
    fn test_pty_example_with_timeout() {
        let cli = Cli::parse_from(["pty", "--host", "example.com", "--timeout", "30"]);
        assert_eq!(cli.timeout, Some(30));
    }
}
