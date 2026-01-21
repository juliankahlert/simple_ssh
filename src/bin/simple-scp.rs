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
#[command(name = "simple-scp")]
#[command(author = "Julian Kahlert")]
#[command(version = "0.1.1")]
#[command(about = "A simple SCP client for file transfer", long_about = None)]
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

    #[arg(required = true)]
    #[arg(help = "Local file to upload")]
    local: PathBuf,

    #[arg(required = true)]
    #[arg(help = "Remote destination path")]
    remote: String,
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

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();

    let mut session = Session::init()
        .with_host(&args.host)
        .with_user(&args.user)
        .with_port(args.port);

    if let Some(scope) = &args.scope {
        session = session.with_scope(scope);
    }

    match args.auth {
        Some(AuthMethod::Password) => {
            let passwd = args.passwd.ok_or_else(|| {
                anyhow!("Password authentication requires --passwd option")
            })?;
            session = session.with_passwd(&passwd);
        }
        Some(AuthMethod::Key) => {
            let key = args.key.ok_or_else(|| {
                anyhow!("Key authentication requires --key option")
            })?;
            session = session.with_key(key);
        }
        Some(AuthMethod::None) => {}
        None => {
            if let Some(key) = args.key {
                session = session.with_key(key);
            } else if let Some(passwd) = args.passwd {
                session = session.with_passwd(&passwd);
            }
        }
    }

    let session = session.build()?;

    let mut ssh = match timeout(Duration::from_secs(30), session.connect()).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return Err(anyhow!("Connection failed: {}", e)),
        Err(_) => return Err(anyhow!("Connection timed out")),
    };

    let local_path = args.local.to_string_lossy();
    println!("Transferring '{}' to '{}@{}:{}'", local_path, args.user, args.host, args.remote);

    match timeout(Duration::from_secs(3000), ssh.scp(&local_path, &args.remote)).await {
        Ok(Ok(())) => {
            println!("File transferred successfully.");
        }
        Ok(Err(e)) => {
            return Err(anyhow!("SCP transfer failed: {}", e));
        }
        Err(_) => {
            return Err(anyhow!("SCP transfer timed out"));
        }
    }

    ssh.close().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv6Addr;

    #[test]
    fn test_args_parsing_basic() {
        let args = Args::parse_from(&[
            "simple-scp",
            "-H", "localhost",
            "/local/file.txt",
            "/remote/path.txt",
        ]);
        assert_eq!(args.host, "localhost");
        assert_eq!(args.user, "root");
        assert_eq!(args.port, 22);
        assert_eq!(args.local, PathBuf::from("/local/file.txt"));
        assert_eq!(args.remote, "/remote/path.txt");
    }

    #[test]
    fn test_args_parsing_with_options() {
        let args = Args::parse_from(&[
            "simple-scp",
            "-H", "192.168.1.1",
            "-u", "admin",
            "-p", "2222",
            "-P", "secret",
            "/local/file.txt",
            "/remote/path.txt",
        ]);
        assert_eq!(args.host, "192.168.1.1");
        assert_eq!(args.user, "admin");
        assert_eq!(args.port, 2222);
        assert_eq!(args.passwd, Some("secret".to_string()));
        assert_eq!(args.local, PathBuf::from("/local/file.txt"));
        assert_eq!(args.remote, "/remote/path.txt");
    }

    #[test]
    fn test_args_parsing_with_key() {
        let args = Args::parse_from(&[
            "simple-scp",
            "-H", "server.example.com",
            "-i", "/path/to/key",
            "/local/file.txt",
            "/remote/path.txt",
        ]);
        assert_eq!(args.key, Some(PathBuf::from("/path/to/key")));
        assert_eq!(args.local, PathBuf::from("/local/file.txt"));
        assert_eq!(args.remote, "/remote/path.txt");
    }

    #[test]
    fn test_args_parsing_with_scope() {
        let args = Args::parse_from(&[
            "simple-scp",
            "-H", "fe80::1",
            "--scope", "eth0",
            "/local/file.txt",
            "/remote/path.txt",
        ]);
        assert_eq!(args.scope, Some("eth0".to_string()));
        assert_eq!(args.local, PathBuf::from("/local/file.txt"));
        assert_eq!(args.remote, "/remote/path.txt");
    }

    #[test]
    fn test_args_parsing_auth_method() {
        let args = Args::parse_from(&[
            "simple-scp",
            "-H", "server.example.com",
            "--auth", "key",
            "/local/file.txt",
            "/remote/path.txt",
        ]);
        assert_eq!(args.auth, Some(AuthMethod::Key));
    }

    #[test]
    fn test_args_parsing_default_user() {
        let args = Args::parse_from(&[
            "simple-scp",
            "-H", "localhost",
            "/local/file.txt",
            "/remote/path.txt",
        ]);
        assert_eq!(args.user, "root");
    }

    #[test]
    fn test_args_parsing_default_port() {
        let args = Args::parse_from(&[
            "simple-scp",
            "-H", "localhost",
            "/local/file.txt",
            "/remote/path.txt",
        ]);
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
    fn test_auth_method_enum() {
        assert_eq!(AuthMethod::Password, AuthMethod::Password);
        assert_eq!(AuthMethod::Key, AuthMethod::Key);
        assert_eq!(AuthMethod::None, AuthMethod::None);
    }

    #[test]
    fn test_pathbuf_from_string() {
        let path = PathBuf::from("/local/file.txt");
        assert_eq!(path.to_string_lossy(), "/local/file.txt");
    }

    #[test]
    fn test_pathbuf_display() {
        let args = Args::parse_from(&[
            "simple-scp",
            "-H", "localhost",
            "/local/dir/file.txt",
            "/remote/path.txt",
        ]);
        assert_eq!(args.local.to_string_lossy(), "/local/dir/file.txt");
    }

    #[test]
    fn test_args_parsing_remote_path_with_spaces() {
        let args = Args::parse_from(&[
            "simple-scp",
            "-H", "localhost",
            "/local/file.txt",
            "/remote/path/with spaces/file.txt",
        ]);
        assert_eq!(args.remote, "/remote/path/with spaces/file.txt");
    }
}
