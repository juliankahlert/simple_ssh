/*
 * MIT License
 *
 * Copyright (c) 2025 Julian Kahlert
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

use anyhow::{anyhow, Result};
use clap::{Parser, ValueEnum};
use std::path::PathBuf;
use tokio::time::{timeout, Duration};

use simple_ssh::Session;

#[derive(Debug, Parser, Clone, PartialEq)]
#[command(name = "simple-ssh")]
#[command(author = "Julian Kahlert")]
#[command(version = "0.1.1")]
#[command(about = "A simple SSH client with PTY support", long_about = None)]
struct Args {
    #[arg(short = 'H', long)]
    #[arg(help = "SSH host to connect to")]
    host: String,

    #[arg(short, long, default_value = "root")]
    #[arg(help = "SSH username")]
    user: String,

    #[arg(short = 'P', long)]
    #[arg(help = "SSH password")]
    passwd: Option<String>,

    #[arg(short = 'i', long)]
    #[arg(help = "Path to private key file")]
    key: Option<PathBuf>,

    #[arg(short, long, default_value = "22")]
    #[arg(help = "SSH port")]
    port: u16,

    #[arg(long)]
    #[arg(help = "IPv6 scope ID (e.g., interface name or number)")]
    scope: Option<String>,

    #[arg(short, long, value_enum)]
    #[arg(help = "Authentication method")]
    auth: Option<AuthMethod>,

    #[arg(trailing_var_arg = true)]
    #[arg(allow_hyphen_values = true)]
    #[arg(help = "Command to execute (if not provided, opens interactive shell)")]
    command: Vec<String>,
}

#[derive(Debug, Clone, ValueEnum, PartialEq)]
enum AuthMethod {
    #[value(name = "password")]
    Password,
    #[value(name = "key")]
    Key,
    #[value(name = "none")]
    None,
}

fn build_session_from_args(args: &Args) -> Result<Session> {
    let mut session = Session::init()
        .with_host(&args.host)
        .with_user(&args.user)
        .with_port(args.port);

    if let Some(scope) = &args.scope {
        session = session.with_scope(scope);
    }

    match args.auth {
        Some(AuthMethod::Password) => {
            let passwd = args
                .passwd
                .as_ref()
                .ok_or_else(|| anyhow!("Password authentication requires --passwd option"))?;
            session = session.with_passwd(passwd);
        }
        Some(AuthMethod::Key) => {
            let key = args
                .key
                .as_ref()
                .ok_or_else(|| anyhow!("Key authentication requires --key option"))?;
            session = session.with_key(key.clone());
        }
        Some(AuthMethod::None) => {}
        None => {
            if let Some(key) = &args.key {
                session = session.with_key(key.clone());
            } else if let Some(passwd) = &args.passwd {
                session = session.with_passwd(passwd);
            }
        }
    }

    session.build()
}

fn command_from_args(args: &Args) -> String {
    args.command.join(" ")
}

fn has_command(args: &Args) -> bool {
    !args.command.is_empty()
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();
    let session = build_session_from_args(&args)?;

    let mut ssh = match timeout(Duration::from_secs(30), session.connect()).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return Err(anyhow!("Connection failed: {}", e)),
        Err(_) => return Err(anyhow!("Connection timed out")),
    };

    if has_command(&args) {
        non_interactive(&mut ssh, &command_from_args(&args)).await?;
    } else {
        interactive_shell(&mut ssh).await?;
    }

    ssh.close().await?;
    Ok(())
}

async fn interactive_shell(ssh: &mut Session) -> Result<u32> {
    let exit_code = ssh.pty().await?;
    println!("\r\nConnection closed with exit code: {}", exit_code);
    Ok(exit_code)
}

async fn non_interactive(ssh: &mut Session, command: &str) -> Result<u32> {
    let exit_code = ssh.cmd(command).await?;
    Ok(exit_code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv6Addr;

    #[test]
    fn test_args_parsing_basic() {
        let args = Args::parse_from(&["simple-ssh", "-H", "localhost"]);
        assert_eq!(args.host, "localhost");
        assert_eq!(args.user, "root");
        assert_eq!(args.port, 22);
    }

    #[test]
    fn test_args_parsing_with_options() {
        let args = Args::parse_from(&[
            "simple-ssh",
            "-H",
            "192.168.1.1",
            "-u",
            "admin",
            "-p",
            "2222",
            "-P",
            "secret",
        ]);
        assert_eq!(args.host, "192.168.1.1");
        assert_eq!(args.user, "admin");
        assert_eq!(args.port, 2222);
        assert_eq!(args.passwd, Some("secret".to_string()));
    }

    #[test]
    fn test_args_parsing_with_command() {
        let args = Args::parse_from(&[
            "simple-ssh",
            "-H",
            "server.example.com",
            "-u",
            "user",
            "echo",
            "hello",
            "world",
        ]);
        assert_eq!(args.command, vec!["echo", "hello", "world"]);
    }

    #[test]
    fn test_args_parsing_with_scope() {
        let args = Args::parse_from(&["simple-ssh", "-H", "fe80::1", "--scope", "eth0"]);
        assert_eq!(args.scope, Some("eth0".to_string()));
    }

    #[test]
    fn test_args_parsing_auth_method() {
        let args = Args::parse_from(&["simple-ssh", "-H", "server.example.com", "--auth", "key"]);
        assert_eq!(args.auth, Some(AuthMethod::Key));
    }

    #[test]
    fn test_args_parsing_default_user() {
        let args = Args::parse_from(&["simple-ssh", "-H", "localhost"]);
        assert_eq!(args.user, "root");
    }

    #[test]
    fn test_args_parsing_default_port() {
        let args = Args::parse_from(&["simple-ssh", "-H", "localhost"]);
        assert_eq!(args.port, 22);
    }

    #[test]
    fn test_ipv6_link_local_format() {
        let addr: Ipv6Addr = "fe80::1".parse().unwrap();
        assert!(addr.is_unicast_link_local());
    }

    #[test]
    fn test_scope_id_append() {
        let host = "fe80::1";
        let scope = "eth0";
        let host_with_scope = format!("{}%{}", host, scope);
        assert_eq!(host_with_scope, "fe80::1%eth0");
    }

    #[test]
    fn test_command_join() {
        let cmd = vec!["echo", "hello", "world"]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        assert_eq!(cmd.join(" "), "echo hello world");
    }

    #[test]
    fn test_auth_method_enum() {
        assert_eq!(AuthMethod::Password, AuthMethod::Password);
        assert_eq!(AuthMethod::Key, AuthMethod::Key);
        assert_eq!(AuthMethod::None, AuthMethod::None);
    }

    #[test]
    fn test_hyphen_command_value() {
        let args = Args::parse_from(&["simple-ssh", "-H", "localhost", "--", "-c", "echo hello"]);
        assert_eq!(args.command, vec!["-c", "echo hello"]);
    }

    #[test]
    fn test_empty_command_vec() {
        let args = Args::parse_from(&["simple-ssh", "-H", "localhost"]);
        assert!(args.command.is_empty());
    }

    #[test]
    fn test_build_session_from_args_password() {
        let args = Args::parse_from(&[
            "simple-ssh",
            "-H",
            "testhost",
            "-u",
            "testuser",
            "-p",
            "2222",
            "-P",
            "password",
        ]);
        let session = build_session_from_args(&args);
        assert!(session.is_ok());
    }

    #[test]
    fn test_build_session_from_args_key() {
        let args = Args::parse_from(&[
            "simple-ssh",
            "-H",
            "testhost",
            "-u",
            "testuser",
            "-k",
            "/path/to/key",
        ]);
        let session = build_session_from_args(&args);
        assert!(session.is_ok());
    }

    #[test]
    fn test_build_session_from_args_with_scope() {
        let args = Args::parse_from(&[
            "simple-ssh",
            "-H",
            "fe80::1",
            "--scope",
            "eth0",
            "-P",
            "pass",
        ]);
        let session = build_session_from_args(&args);
        assert!(session.is_ok());
    }

    #[test]
    fn test_build_session_from_args_no_auth() {
        let args = Args::parse_from(&["simple-ssh", "-H", "testhost", "-u", "testuser"]);
        let session = build_session_from_args(&args);
        assert!(session.is_ok());
    }

    #[test]
    fn test_build_session_auth_password_explicit() {
        let args = Args::parse_from(&[
            "simple-ssh",
            "-H",
            "testhost",
            "--auth",
            "password",
            "-P",
            "mypass",
        ]);
        let session = build_session_from_args(&args);
        assert!(session.is_ok());
    }

    #[test]
    fn test_build_session_auth_key_explicit() {
        let args = Args::parse_from(&[
            "simple-ssh",
            "-H",
            "testhost",
            "--auth",
            "key",
            "-i",
            "/path/to/key",
        ]);
        let session = build_session_from_args(&args);
        assert!(session.is_ok());
    }

    #[test]
    fn test_build_session_auth_none_explicit() {
        let args = Args::parse_from(&["simple-ssh", "-H", "testhost", "--auth", "none"]);
        let session = build_session_from_args(&args);
        assert!(session.is_ok());
    }

    #[test]
    fn test_build_session_error_missing_password() {
        let args = Args::parse_from(&["simple-ssh", "-H", "testhost", "--auth", "password"]);
        let session = build_session_from_args(&args);
        assert!(session.is_err());
        if let Err(e) = session {
            assert!(e.to_string().contains("Password authentication requires"));
        }
    }

    #[test]
    fn test_build_session_error_missing_key() {
        let args = Args::parse_from(&["simple-ssh", "-H", "testhost", "--auth", "key"]);
        let session = build_session_from_args(&args);
        assert!(session.is_err());
        if let Err(e) = session {
            assert!(e.to_string().contains("Key authentication requires"));
        }
    }

    #[test]
    fn test_command_from_args_empty() {
        let args = Args::parse_from(&["simple-ssh", "-H", "localhost"]);
        assert_eq!(command_from_args(&args), "");
    }

    #[test]
    fn test_command_from_args_single() {
        let args = Args::parse_from(&["simple-ssh", "-H", "localhost", "ls"]);
        assert_eq!(command_from_args(&args), "ls");
    }

    #[test]
    fn test_command_from_args_multiple() {
        let args = Args::parse_from(&["simple-ssh", "-H", "localhost", "echo", "hello", "world"]);
        assert_eq!(command_from_args(&args), "echo hello world");
    }

    #[test]
    fn test_has_command_true() {
        let args = Args::parse_from(&["simple-ssh", "-H", "localhost", "ls"]);
        assert!(has_command(&args));
    }

    #[test]
    fn test_has_command_false() {
        let args = Args::parse_from(&["simple-ssh", "-H", "localhost"]);
        assert!(!has_command(&args));
    }
}
