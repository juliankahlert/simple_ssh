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
