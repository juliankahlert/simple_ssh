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
