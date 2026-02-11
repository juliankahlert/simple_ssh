use anyhow::Result;
use simple_ssh::Session;

mod cli;
use cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::with("host")?.and("scope")?.and("user")?.and("passwd")?;

    let host = args.host.expect("host is required");
    let user = args.user.expect("user is required");
    let passwd = args.passwd.expect("password is required");
    let port = args.port.unwrap_or(22);

    let mut session = Session::init()
        .with_host(&host)
        .with_user(&user)
        .with_passwd(&passwd)
        .with_port(port);

    if let Some(scope) = &args.scope {
        session = session.with_scope(scope);
    }

    let mut ssh = session.build()?.connect().await?;

    let code = ssh.cmd("uname -a").await?;
    println!("Exit code: {}", code);

    ssh.close().await?;
    Ok(())
}
