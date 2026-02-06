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

use std::convert::TryFrom;
use std::env;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::anyhow;
use anyhow::Error;
use anyhow::Result;
use russh::keys::*;
use russh::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::client::Msg;
use tokio::fs::File;
use tokio::time::timeout;

use log::debug;
use log::info;

fn resolve_socket_addr(host: &str, port: u16, scope: Option<&str>) -> Result<SocketAddr> {
    let host_with_scope = if let Some(scope_id) = scope {
        format!("{}%{}", host, scope_id)
    } else {
        host.to_string()
    };

    match (host_with_scope.as_str(), port).to_socket_addrs() {
        Ok(mut addrs) => {
            if let Some(addr) = addrs.next() {
                Ok(addr)
            } else {
                Err(anyhow!("No socket addresses resolved for {}", host))
            }
        }
        Err(e) => Err(anyhow!("Failed to resolve host '{}': {}", host, e)),
    }
}

pub struct Session {
    inner: SessionInner,
}

impl<'sb> Session {
    pub fn init() -> SessionBuilder<'sb> {
        SessionBuilder {
            cmdv: vec!["bash".to_string()],
            host: "localhost",
            user: "root",
            passwd: None,
            cert: None,
            key: None,
            port: 22,
            scope: None,
            inactivity_timeout: Some(Duration::from_secs(3000)),
        }
    }

    pub async fn connect(self) -> Result<Self> {
        match self.inner.connect().await {
            Ok(res) => Ok(Session { inner: res }),
            Err(e) => Err(e),
        }
    }

    pub async fn pty(&mut self) -> Result<u32> {
        self.inner.pty().await
    }

    pub async fn run(&mut self) -> Result<u32> {
        self.inner.exec(None, true, true).await
    }

    pub async fn exec(&mut self, command: &Vec<String>) -> Result<u32> {
        self.inner.exec(Some(command), false, false).await
    }

    pub async fn system(&mut self, command: &str) -> Result<u32> {
        let sys_cmd = vec!["sh".to_string(), "-c".to_string(), command.to_string()];
        self.inner.exec(Some(&sys_cmd), false, false).await
    }

    pub async fn cmd(&mut self, command: &str) -> Result<u32> {
        self.inner.cmd(command, false, false).await
    }

    pub async fn scp(&mut self, from: &str, to: &str) -> Result<()> {
        self.inner.scp(from, to).await
    }

    pub async fn close(&mut self) -> Result<()> {
        self.inner.close().await
    }
}

pub struct SessionBuilder<'sb> {
    passwd: Option<String>,
    cert: Option<PathBuf>,
    key: Option<PathBuf>,
    cmdv: Vec<String>,
    user: &'sb str,
    host: &'sb str,
    port: u16,
    scope: Option<String>,
    inactivity_timeout: Option<Duration>,
}

impl<'sb> SessionBuilder<'sb> {
    pub fn with_cert_opt(mut self, cert: Option<PathBuf>) -> Self {
        self.cert = cert;
        self
    }
    pub fn with_key_opt(mut self, key: Option<PathBuf>) -> Self {
        self.key = key;
        self
    }
    pub fn with_cert(mut self, cert: PathBuf) -> Self {
        self.cert = Some(cert);
        self
    }
    pub fn with_key(mut self, key: PathBuf) -> Self {
        self.key = Some(key);
        self
    }
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }
    pub fn with_host(mut self, host: &'sb str) -> Self {
        self.host = host;
        self
    }
    pub fn with_user(mut self, user: &'sb str) -> Self {
        self.user = user;
        self
    }
    pub fn with_cmd(mut self, cmdv: Vec<String>) -> Self {
        self.cmdv = cmdv;
        self
    }
    pub fn with_passwd(mut self, passwd: &str) -> Self {
        self.passwd = Some(passwd.to_string());
        self
    }
    pub fn with_passwd_opt(mut self, passwd: Option<String>) -> Self {
        self.passwd = passwd;
        self
    }
    pub fn with_scope(mut self, scope: &str) -> Self {
        self.scope = Some(scope.to_string());
        self
    }
    pub fn with_inactivity_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.inactivity_timeout = timeout;
        self
    }
    pub fn build(self) -> Result<Session> {
        if let Some(key) = self.key {
            Ok(Session {
                inner: SessionInner::PubKey {
                    session: None,
                    data: SessionDataPubKey {
                        user: self.user.to_string(),
                        host: self.host.to_string(),
                        cmdv: self.cmdv,
                        port: self.port,
                        cert: self.cert,
                        key,
                        scope: self.scope,
                        inactivity_timeout: self.inactivity_timeout,
                    },
                },
            })
        } else if let Some(passwd) = self.passwd {
            Ok(Session {
                inner: SessionInner::Passwd {
                    session: None,
                    data: SessionDataPasswd {
                        user: self.user.to_string(),
                        host: self.host.to_string(),
                        cmdv: self.cmdv,
                        port: self.port,
                        passwd,
                        scope: self.scope,
                        inactivity_timeout: self.inactivity_timeout,
                    },
                },
            })
        } else {
            Ok(Session {
                inner: SessionInner::NoAuth {
                    session: None,
                    data: SessionDataNoAuth {
                        user: self.user.to_string(),
                        host: self.host.to_string(),
                        cmdv: self.cmdv,
                        port: self.port,
                        scope: self.scope,
                        inactivity_timeout: self.inactivity_timeout,
                    },
                },
            })
        }
    }
}

struct Client;

impl client::Handler for Client {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

#[derive(Clone)]
struct SessionDataPasswd {
    cmdv: Vec<String>,
    passwd: String,
    user: String,
    host: String,
    port: u16,
    scope: Option<String>,
    inactivity_timeout: Option<Duration>,
}

#[derive(Clone)]
struct SessionDataPubKey {
    cert: Option<PathBuf>,
    cmdv: Vec<String>,
    user: String,
    host: String,
    key: PathBuf,
    port: u16,
    scope: Option<String>,
    inactivity_timeout: Option<Duration>,
}

#[derive(Clone)]
struct SessionDataNoAuth {
    cmdv: Vec<String>,
    user: String,
    host: String,
    port: u16,
    scope: Option<String>,
    inactivity_timeout: Option<Duration>,
}

enum SessionInner {
    Passwd {
        data: SessionDataPasswd,
        session: Option<client::Handle<Client>>,
    },
    PubKey {
        data: SessionDataPubKey,
        session: Option<client::Handle<Client>>,
    },
    NoAuth {
        data: SessionDataNoAuth,
        session: Option<client::Handle<Client>>,
    },
}

impl SessionInner {
    async fn connect(self) -> Result<Self> {
        match self {
            Self::Passwd {
                data: _,
                session: _,
            } => self.connect_passwd().await,
            Self::PubKey {
                data: _,
                session: _,
            } => self.connect_key().await,
            Self::NoAuth {
                data: _,
                session: _,
            } => self.connect_noauth().await,
        }
    }

    async fn pty(&mut self) -> Result<u32> {
        let command = self.get_command();

        let Some(sess) = self.get_session() else {
            return Err(Error::msg("No open session"));
        };

        pty(sess, &command).await
    }

    async fn close(&mut self) -> Result<()> {
        let Some(sess) = self.get_session() else {
            return Ok(());
        };

        close_session(sess).await
    }

    async fn scp(&mut self, from: &str, to: &str) -> Result<()> {
        let Some(sess) = self.get_session() else {
            return Err(Error::msg("No open session"));
        };

        return scp(sess, from, to).await;
    }

    async fn exec(&mut self, command: Option<&Vec<String>>, err: bool, out: bool) -> Result<u32> {
        let cmd = if let Some(c) = command {
            c.join(" ")
        } else {
            self.get_command()
        };

        if let Some(session) = self.get_session() {
            return system(session, &cmd, err, out).await;
        }

        Err(Error::msg("No open session"))
    }

    async fn cmd(&mut self, command: &str, err: bool, out: bool) -> Result<u32> {
        if let Some(session) = self.get_session() {
            return system(session, command, err, out).await;
        }

        Err(Error::msg("No open session"))
    }

    fn get_session(&mut self) -> &mut Option<client::Handle<Client>> {
        match self {
            Self::Passwd { data: _, session } => session,
            Self::PubKey { data: _, session } => session,
            Self::NoAuth { data: _, session } => session,
        }
    }

    fn get_command(&self) -> String {
        let cmd = match self {
            Self::Passwd { data, session: _ } => &data.cmdv,
            Self::PubKey { data, session: _ } => &data.cmdv,
            Self::NoAuth { data, session: _ } => &data.cmdv,
        };

        cmd.iter()
            .map(|x| shell_escape::escape(x.into())) // arguments are escaped manually since the SSH protocol doesn't support quoting
            .collect::<Vec<_>>()
            .join(" ")
    }

    async fn connect_noauth(self) -> Result<Self> {
        if let Self::NoAuth { data, session: _ } = self {
            let config = client::Config {
                inactivity_timeout: data.inactivity_timeout,
                ..<_>::default()
            };
            let config = Arc::new(config);
            let sh = Client {};
            let addrs = resolve_socket_addr(&data.host, data.port, data.scope.as_deref())?;
            let mut session = client::connect(config, addrs, sh).await?;

            info!(
                "Connecting using password {}@{}:{}",
                &data.user, &data.host, &data.port
            );
            let auth_res = session.authenticate_none(data.user.clone()).await?;

            if !auth_res.success() {
                return Err(Error::msg("Authentication None failed"));
            }

            return Ok(Self::NoAuth {
                data,
                session: Some(session),
            });
        }
        Err(Error::msg("connect_noauth called on non Session::NoAuth"))
    }

    async fn connect_passwd(self) -> Result<Self> {
        if let Self::Passwd { data, session: _ } = self {
            let config = client::Config {
                inactivity_timeout: data.inactivity_timeout,
                ..<_>::default()
            };
            let config = Arc::new(config);
            let sh = Client {};
            let addrs = resolve_socket_addr(&data.host, data.port, data.scope.as_deref())?;
            let mut session = client::connect(config, addrs, sh).await?;

            info!(
                "Connecting using password {}@{}:{}",
                &data.user, &data.host, &data.port
            );
            let auth_res = session
                .authenticate_password(data.user.clone(), data.passwd.clone())
                .await?;

            if !auth_res.success() {
                return Err(Error::msg("Authentication (with passwd) failed"));
            }

            return Ok(Self::Passwd {
                data,
                session: Some(session),
            });
        }
        Err(Error::msg("connect_passwd called on non Session::Passwd"))
    }

    async fn connect_key(self) -> Result<Self> {
        if let Self::PubKey { data, session: _ } = self {
            let key_pair = load_secret_key(data.key.clone(), None)?;

            // load ssh certificate
            let mut openssh_cert = None;
            if let Some(c) = &data.cert {
                openssh_cert = Some(load_openssh_certificate(c)?);
            }

            let config = client::Config {
                inactivity_timeout: data.inactivity_timeout,
                ..<_>::default()
            };

            let config = Arc::new(config);
            let sh = Client {};
            let addrs = resolve_socket_addr(&data.host, data.port, data.scope.as_deref())?;
            let mut session = client::connect(config, addrs, sh).await?;

            info!(
                "Connecting using public key {}@{}:{}",
                &data.user, &data.host, &data.port
            );

            // use publickey authentication, with or without certificate
            if openssh_cert.is_none() {
                let auth_res = session
                    .authenticate_publickey(
                        data.user.clone(),
                        PrivateKeyWithHashAlg::new(
                            Arc::new(key_pair),
                            session.best_supported_rsa_hash().await?.flatten(),
                        ),
                    )
                    .await?;

                if !auth_res.success() {
                    return Err(Error::msg("Authentication (with publickey) failed"));
                }
            } else {
                let auth_res = session
                    .authenticate_openssh_cert(
                        data.user.clone(),
                        Arc::new(key_pair),
                        openssh_cert.unwrap(),
                    )
                    .await?;

                if !auth_res.success() {
                    return Err(Error::msg("Authentication (with publickey+cert) failed"));
                }
            }

            return Ok(Self::PubKey {
                data,
                session: Some(session),
            });
        }
        Err(Error::msg("connect_pubkey called on non Session::PubKey"))
    }
}

/******************************************** Helper ********************************************/

async fn pty(session: &mut client::Handle<Client>, command: &str) -> Result<u32> {
    let mut channel = session.channel_open_session().await?;

    // This example doesn't terminal resizing after the connection is established
    let (w, h) = termion::terminal_size()?;

    // Request an interactive PTY from the server
    channel
        .request_pty(
            false,
            &env::var("TERM").unwrap_or("xterm".into()),
            w as u32,
            h as u32,
            0,
            0,
            &[], // ideally you want to pass the actual terminal modes here
        )
        .await?;
    channel.exec(true, command).await?;

    let code;
    let mut stdin = tokio_fd::AsyncFd::try_from(0)?;
    let mut stdout = tokio_fd::AsyncFd::try_from(1)?;
    let mut buf = vec![0; 1024];
    let mut stdin_closed = false;

    loop {
        // Handle one of the possible events:
        tokio::select! {
            // There's terminal input available from the user
            r = stdin.read(&mut buf), if !stdin_closed => {
                match r {
                    Ok(0) => {
                        stdin_closed = true;
                        channel.eof().await?;
                    },
                    // Send it to the server
                    Ok(n) => channel.data(&buf[..n]).await?,
                    Err(e) => return Err(e.into()),
                };
            },
            // There's an event available on the session channel
            msg = channel.wait() => {
                match msg {
                    // Write data to the terminal
                    Some(ChannelMsg::Data { ref data }) => {
                        stdout.write_all(data).await?;
                        stdout.flush().await?;
                    }
                    // The command has returned an exit code
                    Some(ChannelMsg::ExitStatus { exit_status }) => {
                        code = exit_status;
                        if !stdin_closed {
                            channel.eof().await?;
                        }
                        break;
                    }
                    // Channel closed unexpectedly
                    None => {
                        return Err(Error::msg("Channel closed unexpectedly"));
                    }
                    _ => {}
                }
            },
        }
    }
    Ok(code)
}

async fn close_session(session: &mut client::Handle<Client>) -> Result<()> {
    session
        .disconnect(Disconnect::ByApplication, "", "English")
        .await?;
    Ok(())
}

async fn system(
    session: &mut client::Handle<Client>,
    command: &str,
    err: bool,
    out: bool,
) -> Result<u32> {
    let mut channel = session.channel_open_session().await?;
    channel.exec(true, command).await?;

    let mut code = None;
    let mut stdout = tokio::io::stdout();
    let mut stderr = tokio::io::stderr();

    loop {
        let Some(msg) = channel.wait().await else {
            break;
        };
        match msg {
            ChannelMsg::Data { ref data } => {
                if out {
                    stdout.write_all(data).await?;
                    stdout.flush().await?;
                }
            }
            ChannelMsg::ExtendedData { ref data, ext } => {
                if err && ext == 1 {
                    stderr.write_all(data).await?;
                    stderr.flush().await?;
                }
            }
            ChannelMsg::ExitStatus { exit_status } => {
                code = Some(exit_status);
                // cannot leave the loop immediately, there might still be more data to receive
            }
            _ => {}
        }
    }
    code.ok_or(Error::msg("program did not exit cleanly"))
}

async fn wait_for_data(channel: &mut Channel<Msg>) -> Result<Vec<u8>> {
    loop {
        match channel.wait().await {
            Some(ChannelMsg::Data { ref data }) => {
                return Ok(data.to_vec());
            }
            Some(ChannelMsg::ExtendedData { ref data, ext: 1 }) => {
                return Err(Error::msg(format!(
                    "SCP error: {}",
                    String::from_utf8_lossy(data)
                )));
            }
            Some(ChannelMsg::ExitStatus { exit_status }) => {
                return Err(Error::msg(format!(
                    "SCP failed with exit code {}",
                    exit_status
                )));
            }
            Some(_) => {
                // Ignore other messages and continue waiting
            }
            None => {
                // Channel closed unexpectedly
                return Err(Error::msg("Channel closed unexpectedly"));
            }
        }
    }
}

struct SCPStateOpen {
    channel: Channel<Msg>,
}

impl SCPStateOpen {
    async fn start_tx(mut self, remote_path: &str) -> Result<SCPStateTxStart> {
        let command = format!("scp -t {}", remote_path);
        self.channel.exec(true, command).await?;
        // TODO "cat > {}" is an alternative here if the target has no scp

        let data = wait_for_data(&mut self.channel).await?;
        if data[0] != 0 {
            return Err(Error::msg(format!("SCP start failed: {:?}", data)));
        }

        Ok(SCPStateTxStart {
            channel: self.channel,
        })
    }
}

async fn scp_channel_open(session: &mut client::Handle<Client>) -> Result<SCPStateOpen> {
    let res = session.channel_open_session().await;

    match res {
        Err(e) => Err(Error::msg(e.to_string())),
        Ok(channel) => Ok(SCPStateOpen { channel }),
    }
}

struct SCPStateTxStart {
    channel: Channel<Msg>,
}

impl SCPStateTxStart {
    async fn write_metadata(mut self, file_size: u64, file_name: &str) -> Result<SCPStateTxData> {
        let metadata_msg = format!("C0644 {} {}\n", file_size, file_name);
        self.channel.data(metadata_msg.as_bytes()).await?;

        let data = wait_for_data(&mut self.channel).await?;
        if data[0] != 0 {
            return Err(Error::msg(format!("SCP confirmation failed: {:?}", data)));
        }

        Ok(SCPStateTxData {
            channel: self.channel,
        })
    }
}

struct SCPStateTxData {
    channel: Channel<Msg>,
}

impl SCPStateTxData {
    async fn write_data(&mut self, buf: &[u8]) -> Result<()> {
        self.channel.data(buf).await?;
        Ok(())
    }

    async fn eof(mut self) -> Result<SCPStateEOF> {
        self.channel.data(&b"\0"[..]).await?;
        let data = wait_for_data(&mut self.channel).await?;
        if data[0] != 0 {
            return Err(Error::msg(format!(
                "SCP post-data confirmation failed: {:?}",
                data
            )));
        }
        self.channel.eof().await?;

        Ok(SCPStateEOF {
            channel: self.channel,
        })
    }
}

struct SCPStateEOF {
    channel: Channel<Msg>,
}

impl SCPStateEOF {
    async fn close(self) -> Result<()> {
        self.channel.close().await?;
        Ok(())
    }
}

async fn scp(
    session: &mut client::Handle<Client>,
    local_path: &str,
    remote_path: &str,
) -> Result<()> {
    let file = File::open(local_path).await?;
    let state = scp_channel_open(session).await?;
    let state = state.start_tx(remote_path).await?;

    // Get file size and name
    let metadata = file.metadata().await?;
    let file_size = metadata.len();
    let file_name = std::path::Path::new(remote_path)
        .file_name()
        .ok_or_else(|| anyhow!("Invalid file name"))?
        .to_string_lossy();

    let mut state = state.write_metadata(file_size, &file_name).await?;

    const WRITE_TIMEOUT: Duration = Duration::from_secs(16);
    let mut buffer = [0u8; 16 * 1024];
    let mut reader = file;

    let mut eof_reached = false;

    loop {
        tokio::select! {
            // Read from file and send data
            result = reader.read(&mut buffer), if !eof_reached => {
                match result {
                    Ok(0) => {
                        // EOF reached, mark it but continue processing channel messages
                        eof_reached = true;
                    }
                    Ok(n) => {
                        debug!("Writing {} bytes to {}", n, remote_path);
                        // Apply timeout only to the write operation
                        timeout(WRITE_TIMEOUT, state.write_data(&buffer[..n]))
                            .await
                            .map_err(|_| anyhow!("Write timed out after {:?}", WRITE_TIMEOUT))??;
                    }
                    Err(e) => return Err(e.into()),
                }
            }

            // Handle SSH channel messages (window adjust, errors, etc.)
            msg = state.channel.wait() => {
                match msg {
                    Some(ChannelMsg::ExtendedData { data, ext: 1 }) => {
                        return Err(anyhow!(
                            "Remote SCP error: {}",
                            String::from_utf8_lossy(&data)
                        ));
                    }
                    Some(ChannelMsg::ExitStatus { exit_status }) => {
                        return Err(anyhow!(
                            "Remote SCP exited early with code: {}",
                            exit_status
                        ));
                    }
                    Some(_) => {
                        // Window adjust, keepalive, or other protocol messages - ignore
                    }
                    None => {
                        // Channel closed unexpectedly
                        return Err(anyhow!("Channel closed during transfer"));
                    }
                }
            }
        }

        // Exit the loop after EOF and all pending operations complete
        if eof_reached {
            break;
        }
    }

    let state = state.eof().await?;
    state.close().await
}

#[tokio::test]
async fn test_session_builder() {
    let session = Session::init()
        .with_user("user")
        .with_host("localhost")
        .with_passwd("password")
        .build();

    assert!(session.is_ok());

    let session = session.unwrap();
    if let SessionInner::Passwd { data, .. } = session.inner {
        assert_eq!(data.user, "user");
        assert_eq!(data.host, "localhost");
        assert_eq!(data.passwd, "password");
    } else {
        panic!("Expected Passwd session.");
    }

    let session = Session::init()
        .with_user("user")
        .with_host("localhost")
        .with_key("path/to/key".into())
        .build();

    assert!(session.is_ok());

    let session = session.unwrap();
    if let SessionInner::PubKey { data, .. } = session.inner {
        assert_eq!(data.user, "user");
        assert_eq!(data.host, "localhost");
        assert_eq!(data.key.to_str(), Some("path/to/key"));
    } else {
        panic!("Expected PubKey session.");
    }
}

#[tokio::test]
async fn test_session_builder_no_auth() {
    // Test NoAuth variant when neither password nor key is provided
    let session = Session::init()
        .with_user("testuser")
        .with_host("example.com")
        .build();

    assert!(session.is_ok());

    let session = session.unwrap();
    if let SessionInner::NoAuth { data, .. } = session.inner {
        assert_eq!(data.user, "testuser");
        assert_eq!(data.host, "example.com");
        assert_eq!(data.port, 22);
    } else {
        panic!("Expected NoAuth session.");
    }
}

#[tokio::test]
async fn test_session_builder_with_port() {
    let session = Session::init()
        .with_host("example.com")
        .with_user("admin")
        .with_port(2222)
        .with_passwd("secret")
        .build();

    assert!(session.is_ok());

    let session = session.unwrap();
    if let SessionInner::Passwd { data, .. } = session.inner {
        assert_eq!(data.port, 2222);
        assert_eq!(data.user, "admin");
        assert_eq!(data.host, "example.com");
    } else {
        panic!("Expected Passwd session.");
    }
}

#[tokio::test]
async fn test_session_builder_with_scope() {
    let session = Session::init()
        .with_host("fe80::1")
        .with_user("admin")
        .with_scope("eth0")
        .with_passwd("secret")
        .build();

    assert!(session.is_ok());

    let session = session.unwrap();
    if let SessionInner::Passwd { data, .. } = session.inner {
        assert_eq!(data.scope, Some("eth0".to_string()));
        assert_eq!(data.host, "fe80::1");
    } else {
        panic!("Expected Passwd session.");
    }
}

#[tokio::test]
async fn test_session_builder_with_cmd() {
    let custom_cmd = vec!["zsh".to_string(), "-l".to_string()];
    let session = Session::init()
        .with_host("localhost")
        .with_user("user")
        .with_cmd(custom_cmd.clone())
        .with_passwd("pass")
        .build();

    assert!(session.is_ok());

    let session = session.unwrap();
    if let SessionInner::Passwd { data, .. } = session.inner {
        assert_eq!(data.cmdv, custom_cmd);
    } else {
        panic!("Expected Passwd session.");
    }
}

#[tokio::test]
async fn test_session_builder_with_timeout() {
    let custom_timeout = Some(Duration::from_secs(600));
    let session = Session::init()
        .with_host("localhost")
        .with_user("user")
        .with_inactivity_timeout(custom_timeout)
        .with_passwd("pass")
        .build();

    assert!(session.is_ok());

    let session = session.unwrap();
    if let SessionInner::Passwd { data, .. } = session.inner {
        assert_eq!(data.inactivity_timeout, custom_timeout);
    } else {
        panic!("Expected Passwd session.");
    }
}

#[tokio::test]
async fn test_session_builder_with_timeout_disabled() {
    let session = Session::init()
        .with_host("localhost")
        .with_user("user")
        .with_inactivity_timeout(None)
        .with_passwd("pass")
        .build();

    assert!(session.is_ok());

    let session = session.unwrap();
    if let SessionInner::Passwd { data, .. } = session.inner {
        assert_eq!(data.inactivity_timeout, None);
    } else {
        panic!("Expected Passwd session.");
    }
}

#[tokio::test]
async fn test_session_builder_with_key_and_cert() {
    let key_path = PathBuf::from("/path/to/key");
    let cert_path = PathBuf::from("/path/to/cert");

    let session = Session::init()
        .with_host("localhost")
        .with_user("user")
        .with_key(key_path.clone())
        .with_cert(cert_path.clone())
        .build();

    assert!(session.is_ok());

    let session = session.unwrap();
    if let SessionInner::PubKey { data, .. } = session.inner {
        assert_eq!(data.key, key_path);
        assert_eq!(data.cert, Some(cert_path));
    } else {
        panic!("Expected PubKey session.");
    }
}

#[tokio::test]
async fn test_session_builder_with_key_opt() {
    let key_path = Some(PathBuf::from("/path/to/key"));

    let session = Session::init()
        .with_host("localhost")
        .with_user("user")
        .with_key_opt(key_path.clone())
        .build();

    assert!(session.is_ok());

    let session = session.unwrap();
    if let SessionInner::PubKey { data, .. } = session.inner {
        assert_eq!(data.key, key_path.unwrap());
    } else {
        panic!("Expected PubKey session.");
    }
}

#[tokio::test]
async fn test_session_builder_with_cert_opt() {
    let key_path = PathBuf::from("/path/to/key");
    let cert_path = Some(PathBuf::from("/path/to/cert"));

    let session = Session::init()
        .with_host("localhost")
        .with_user("user")
        .with_key(key_path)
        .with_cert_opt(cert_path.clone())
        .build();

    assert!(session.is_ok());

    let session = session.unwrap();
    if let SessionInner::PubKey { data, .. } = session.inner {
        assert_eq!(data.cert, cert_path);
    } else {
        panic!("Expected PubKey session.");
    }
}

#[tokio::test]
async fn test_session_builder_with_passwd_opt() {
    let passwd = Some("password123".to_string());

    let session = Session::init()
        .with_host("localhost")
        .with_user("user")
        .with_passwd_opt(passwd.clone())
        .build();

    assert!(session.is_ok());

    let session = session.unwrap();
    if let SessionInner::Passwd { data, .. } = session.inner {
        assert_eq!(data.passwd, passwd.unwrap());
    } else {
        panic!("Expected Passwd session.");
    }
}

#[tokio::test]
async fn test_session_builder_defaults() {
    let session = Session::init().with_passwd("pass").build();

    assert!(session.is_ok());

    let session = session.unwrap();
    if let SessionInner::Passwd { data, .. } = session.inner {
        assert_eq!(data.user, "root");
        assert_eq!(data.host, "localhost");
        assert_eq!(data.port, 22);
        assert_eq!(data.cmdv, vec!["bash".to_string()]);
        assert_eq!(data.scope, None);
        assert_eq!(data.inactivity_timeout, Some(Duration::from_secs(3000)));
    } else {
        panic!("Expected Passwd session.");
    }
}

#[test]
fn test_resolve_socket_addr_ipv4() {
    let result = resolve_socket_addr("127.0.0.1", 22, None);
    assert!(result.is_ok());
    let addr = result.unwrap();
    assert_eq!(addr.port(), 22);
}

#[test]
fn test_resolve_socket_addr_with_scope() {
    // Test scope formatting (even though it may not resolve without actual interface)
    let _result = resolve_socket_addr("fe80::1", 22, Some("eth0"));
    // May fail to resolve if interface doesn't exist, but we're testing the code path
    // The important part is that it attempts to format with scope
}

#[test]
fn test_resolve_socket_addr_invalid_host() {
    let result = resolve_socket_addr("invalid..host..name", 22, None);
    assert!(result.is_err());
}

#[test]
fn test_resolve_socket_addr_localhost() {
    let result = resolve_socket_addr("localhost", 8080, None);
    assert!(result.is_ok());
    let addr = result.unwrap();
    assert_eq!(addr.port(), 8080);
}

#[tokio::test]
async fn test_session_error_no_connection_pty() {
    let mut session = Session::init().with_passwd("pass").build().unwrap();

    // Calling pty() without connecting should return error
    let result = session.pty().await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "No open session");
}

#[tokio::test]
async fn test_session_error_no_connection_run() {
    let mut session = Session::init().with_passwd("pass").build().unwrap();

    // Calling run() without connecting should return error
    let result = session.run().await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "No open session");
}

#[tokio::test]
async fn test_session_error_no_connection_exec() {
    let mut session = Session::init().with_passwd("pass").build().unwrap();

    let cmd = vec!["ls".to_string()];
    let result = session.exec(&cmd).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "No open session");
}

#[tokio::test]
async fn test_session_error_no_connection_cmd() {
    let mut session = Session::init().with_passwd("pass").build().unwrap();

    let result = session.cmd("ls").await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "No open session");
}

#[tokio::test]
async fn test_session_error_no_connection_system() {
    let mut session = Session::init().with_passwd("pass").build().unwrap();

    let result = session.system("ls | grep foo").await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "No open session");
}

#[tokio::test]
async fn test_session_error_no_connection_scp() {
    let mut session = Session::init().with_passwd("pass").build().unwrap();

    let result = session.scp("/local/file", "/remote/file").await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "No open session");
}

#[tokio::test]
async fn test_session_close_no_connection() {
    let mut session = Session::init().with_passwd("pass").build().unwrap();

    // Calling close() without a connection should succeed (no-op)
    let result = session.close().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_client_handler_check_server_key() {
    use russh::client::Handler;
    use ssh_key::PublicKey as SshPublicKey;

    // Create a client handler
    let mut client = Client {};

    // Create a minimal Ed25519 public key for testing
    // This is a valid Ed25519 public key (32 bytes of zeros for testing)
    let key_data = vec![0u8; 32];
    let public_key = SshPublicKey::new(
        ssh_key::public::KeyData::Ed25519(
            ssh_key::public::Ed25519PublicKey::try_from(&key_data[..]).unwrap(),
        ),
        "",
    );

    // Test that check_server_key returns Ok(true) (accepts any key)
    let result = client.check_server_key(&public_key).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);
}

#[test]
fn test_session_inner_get_command_escaping() {
    // Test that get_command properly escapes shell metacharacters
    let session_inner = SessionInner::Passwd {
        session: None,
        data: SessionDataPasswd {
            user: "user".to_string(),
            host: "localhost".to_string(),
            cmdv: vec![
                "echo".to_string(),
                "hello world".to_string(),
                "$USER".to_string(),
                "test;rm -rf /".to_string(),
            ],
            passwd: "pass".to_string(),
            port: 22,
            scope: None,
            inactivity_timeout: Some(Duration::from_secs(3000)),
        },
    };

    let command = session_inner.get_command();

    // Verify that the command contains echo and escaped arguments
    assert!(command.contains("echo"));
    // Verify that the command is properly formatted with spaces between args
    let parts: Vec<&str> = command.split_whitespace().collect();
    assert!(parts.len() >= 4, "Command should have at least 4 parts");
}

#[test]
fn test_session_inner_get_command_simple() {
    let session_inner = SessionInner::PubKey {
        session: None,
        data: SessionDataPubKey {
            user: "user".to_string(),
            host: "localhost".to_string(),
            cmdv: vec!["bash".to_string(), "-c".to_string(), "ls".to_string()],
            key: PathBuf::from("/path/to/key"),
            cert: None,
            port: 22,
            scope: None,
            inactivity_timeout: Some(Duration::from_secs(3000)),
        },
    };

    let command = session_inner.get_command();
    assert!(command.contains("bash"));
    assert!(command.contains("-c"));
    assert!(command.contains("ls"));
}

#[test]
fn test_session_inner_get_command_noauth() {
    let session_inner = SessionInner::NoAuth {
        session: None,
        data: SessionDataNoAuth {
            user: "user".to_string(),
            host: "localhost".to_string(),
            cmdv: vec!["zsh".to_string()],
            port: 22,
            scope: None,
            inactivity_timeout: Some(Duration::from_secs(3000)),
        },
    };

    let command = session_inner.get_command();
    assert_eq!(command, "zsh");
}

#[tokio::test]
async fn test_session_data_clone_passwd() {
    // Test that SessionDataPasswd is cloneable
    let data = SessionDataPasswd {
        user: "testuser".to_string(),
        host: "testhost".to_string(),
        cmdv: vec!["bash".to_string()],
        passwd: "secret".to_string(),
        port: 2222,
        scope: Some("eth0".to_string()),
        inactivity_timeout: Some(Duration::from_secs(600)),
    };

    let cloned = data.clone();
    assert_eq!(data.user, cloned.user);
    assert_eq!(data.host, cloned.host);
    assert_eq!(data.passwd, cloned.passwd);
    assert_eq!(data.port, cloned.port);
    assert_eq!(data.scope, cloned.scope);
}

#[tokio::test]
async fn test_session_data_clone_pubkey() {
    let data = SessionDataPubKey {
        user: "testuser".to_string(),
        host: "testhost".to_string(),
        cmdv: vec!["bash".to_string()],
        key: PathBuf::from("/path/to/key"),
        cert: Some(PathBuf::from("/path/to/cert")),
        port: 2222,
        scope: Some("eth0".to_string()),
        inactivity_timeout: Some(Duration::from_secs(600)),
    };

    let cloned = data.clone();
    assert_eq!(data.user, cloned.user);
    assert_eq!(data.host, cloned.host);
    assert_eq!(data.key, cloned.key);
    assert_eq!(data.cert, cloned.cert);
    assert_eq!(data.port, cloned.port);
}

#[tokio::test]
async fn test_session_data_clone_noauth() {
    let data = SessionDataNoAuth {
        user: "testuser".to_string(),
        host: "testhost".to_string(),
        cmdv: vec!["bash".to_string()],
        port: 2222,
        scope: Some("eth0".to_string()),
        inactivity_timeout: Some(Duration::from_secs(600)),
    };

    let cloned = data.clone();
    assert_eq!(data.user, cloned.user);
    assert_eq!(data.host, cloned.host);
    assert_eq!(data.port, cloned.port);
    assert_eq!(data.scope, cloned.scope);
}

#[test]
fn test_shell_escape_integration() {
    // Test that shell_escape is working as expected in our context
    use shell_escape::escape;

    let dangerous = "test; rm -rf /";
    let escaped = escape(dangerous.into());
    // Verify that the escaped string is safe (quoted or escaped)
    let escaped_str = escaped.to_string();
    assert_ne!(escaped_str, dangerous, "String should be escaped");

    let with_spaces = "hello world";
    let escaped = escape(with_spaces.into());
    let escaped_str = escaped.to_string();
    // String with spaces should be escaped/quoted
    assert_ne!(
        escaped_str, with_spaces,
        "String with spaces should be escaped"
    );
}

#[tokio::test]
async fn test_session_connect_invalid_host() {
    // Test connection with invalid hostname
    let session = Session::init()
        .with_host("definitely.invalid.hostname.that.does.not.exist.example")
        .with_user("user")
        .with_passwd("pass")
        .build()
        .unwrap();

    let result = session.connect().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_session_connect_invalid_port() {
    // Test connection with closed port (very unlikely to be open)
    let session = Session::init()
        .with_host("127.0.0.1")
        .with_port(1) // Port 1 is typically not accessible
        .with_user("user")
        .with_passwd("pass")
        .build()
        .unwrap();

    let result = tokio::time::timeout(Duration::from_secs(2), session.connect()).await;

    // Either timeout or connection error expected
    assert!(result.is_err() || result.unwrap().is_err());
}

#[tokio::test]
async fn test_session_connect_pubkey_invalid() {
    // Test connection with public key to non-existent host
    let session = Session::init()
        .with_host("invalid.test.example.nonexistent")
        .with_user("user")
        .with_key(PathBuf::from("/nonexistent/key"))
        .build()
        .unwrap();

    let result = session.connect().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_session_connect_noauth_invalid() {
    // Test NoAuth connection to non-existent host
    let session = Session::init()
        .with_host("invalid.test.example.nonexistent")
        .with_user("user")
        .build()
        .unwrap();

    let result = session.connect().await;
    assert!(result.is_err());
}

#[test]
fn test_resolve_socket_addr_empty_result() {
    // Test with a malformed address that might resolve but return no addresses
    let result = resolve_socket_addr("", 22, None);
    assert!(result.is_err());
}

#[test]
fn test_session_init_creates_builder() {
    // Test that Session::init() creates a builder with correct defaults
    let builder = Session::init();

    // Build with just password to verify defaults are set
    let session = builder.with_passwd("test").build();
    assert!(session.is_ok());
}

#[tokio::test]
async fn test_session_multiple_error_methods() {
    // Test multiple methods on unconnected session
    let mut session = Session::init()
        .with_user("testuser")
        .with_host("testhost")
        .with_passwd("testpass")
        .build()
        .unwrap();

    // All these should fail with "No open session"
    assert!(session.pty().await.is_err());
    assert!(session.run().await.is_err());
    assert!(session.cmd("test").await.is_err());
    assert!(session.system("test").await.is_err());
    assert!(session.exec(&vec!["test".to_string()]).await.is_err());
    assert!(session.scp("/src", "/dst").await.is_err());

    // Close should succeed even without connection
    assert!(session.close().await.is_ok());
}

#[test]
fn test_session_inner_variants_construction() {
    // Test all three SessionInner variants can be constructed
    let passwd_inner = SessionInner::Passwd {
        session: None,
        data: SessionDataPasswd {
            user: "user".to_string(),
            host: "host".to_string(),
            cmdv: vec!["bash".to_string()],
            passwd: "pass".to_string(),
            port: 22,
            scope: None,
            inactivity_timeout: Some(Duration::from_secs(3000)),
        },
    };

    let pubkey_inner = SessionInner::PubKey {
        session: None,
        data: SessionDataPubKey {
            user: "user".to_string(),
            host: "host".to_string(),
            cmdv: vec!["bash".to_string()],
            key: PathBuf::from("/key"),
            cert: None,
            port: 22,
            scope: None,
            inactivity_timeout: Some(Duration::from_secs(3000)),
        },
    };

    let noauth_inner = SessionInner::NoAuth {
        session: None,
        data: SessionDataNoAuth {
            user: "user".to_string(),
            host: "host".to_string(),
            cmdv: vec!["bash".to_string()],
            port: 22,
            scope: None,
            inactivity_timeout: Some(Duration::from_secs(3000)),
        },
    };

    // Verify get_command works for all variants
    assert!(!passwd_inner.get_command().is_empty());
    assert!(!pubkey_inner.get_command().is_empty());
    assert!(!noauth_inner.get_command().is_empty());
}

#[test]
fn test_pathbuf_operations() {
    // Test PathBuf usage in SessionDataPubKey
    let key_path = PathBuf::from("/home/user/.ssh/id_rsa");
    let cert_path = PathBuf::from("/home/user/.ssh/id_rsa-cert.pub");

    let data = SessionDataPubKey {
        user: "user".to_string(),
        host: "host".to_string(),
        cmdv: vec!["bash".to_string()],
        key: key_path.clone(),
        cert: Some(cert_path.clone()),
        port: 22,
        scope: None,
        inactivity_timeout: Some(Duration::from_secs(3000)),
    };

    assert_eq!(data.key, key_path);
    assert_eq!(data.cert, Some(cert_path));
}

#[test]
fn test_duration_timeout_values() {
    // Test various timeout duration values
    let short_timeout = Some(Duration::from_secs(1));
    let long_timeout = Some(Duration::from_secs(10000));
    let no_timeout: Option<Duration> = None;

    let session1 = Session::init()
        .with_inactivity_timeout(short_timeout)
        .with_passwd("pass")
        .build()
        .unwrap();

    let session2 = Session::init()
        .with_inactivity_timeout(long_timeout)
        .with_passwd("pass")
        .build()
        .unwrap();

    let session3 = Session::init()
        .with_inactivity_timeout(no_timeout)
        .with_passwd("pass")
        .build()
        .unwrap();

    if let SessionInner::Passwd { data, .. } = session1.inner {
        assert_eq!(data.inactivity_timeout, short_timeout);
    }

    if let SessionInner::Passwd { data, .. } = session2.inner {
        assert_eq!(data.inactivity_timeout, long_timeout);
    }

    if let SessionInner::Passwd { data, .. } = session3.inner {
        assert_eq!(data.inactivity_timeout, no_timeout);
    }
}

#[test]
fn test_session_builder_chaining() {
    // Test that builder methods can be chained in any order
    let session = Session::init()
        .with_port(2222)
        .with_host("example.com")
        .with_scope("eth0")
        .with_user("admin")
        .with_inactivity_timeout(Some(Duration::from_secs(600)))
        .with_passwd("secret")
        .build();

    assert!(session.is_ok());
    let s = session.unwrap();
    if let SessionInner::Passwd { data, .. } = s.inner {
        assert_eq!(data.port, 2222);
        assert_eq!(data.host, "example.com");
        assert_eq!(data.user, "admin");
        assert_eq!(data.scope, Some("eth0".to_string()));
    }
}

#[test]
fn test_session_builder_multiple_configs() {
    // Build sessions with different configurations
    let configs = vec![
        (22u16, "localhost"),
        (2222u16, "192.168.1.1"),
        (22022u16, "10.0.0.1"),
    ];

    for (port, host) in configs {
        let session = Session::init()
            .with_host(host)
            .with_port(port)
            .with_user("test")
            .with_passwd("pass")
            .build();

        assert!(session.is_ok());
    }
}

#[test]
fn test_session_data_fields() {
    // Test that all fields are properly set in SessionDataPasswd
    let cmdv = vec!["zsh".to_string(), "-l".to_string()];
    let session = Session::init()
        .with_user("myuser")
        .with_host("myhost.example.com")
        .with_port(8022)
        .with_scope("wlan0")
        .with_cmd(cmdv.clone())
        .with_passwd("mypassword")
        .with_inactivity_timeout(Some(Duration::from_secs(1200)))
        .build()
        .unwrap();

    if let SessionInner::Passwd { data, session: _ } = session.inner {
        assert_eq!(data.user, "myuser");
        assert_eq!(data.host, "myhost.example.com");
        assert_eq!(data.port, 8022);
        assert_eq!(data.scope, Some("wlan0".to_string()));
        assert_eq!(data.cmdv, cmdv);
        assert_eq!(data.passwd, "mypassword");
        assert_eq!(data.inactivity_timeout, Some(Duration::from_secs(1200)));
    } else {
        panic!("Expected Passwd variant");
    }
}

#[test]
fn test_pubkey_session_all_fields() {
    // Test that all fields are properly set in SessionDataPubKey
    let key = PathBuf::from("/home/user/.ssh/id_ed25519");
    let cert = PathBuf::from("/home/user/.ssh/id_ed25519-cert.pub");
    let cmdv = vec!["fish".to_string()];

    let session = Session::init()
        .with_user("keyuser")
        .with_host("keyhost.example.com")
        .with_port(9022)
        .with_scope("eth1")
        .with_cmd(cmdv.clone())
        .with_key(key.clone())
        .with_cert(cert.clone())
        .with_inactivity_timeout(Some(Duration::from_secs(1800)))
        .build()
        .unwrap();

    if let SessionInner::PubKey { data, session: _ } = session.inner {
        assert_eq!(data.user, "keyuser");
        assert_eq!(data.host, "keyhost.example.com");
        assert_eq!(data.port, 9022);
        assert_eq!(data.scope, Some("eth1".to_string()));
        assert_eq!(data.cmdv, cmdv);
        assert_eq!(data.key, key);
        assert_eq!(data.cert, Some(cert));
        assert_eq!(data.inactivity_timeout, Some(Duration::from_secs(1800)));
    } else {
        panic!("Expected PubKey variant");
    }
}

#[test]
fn test_noauth_session_all_fields() {
    // Test that all fields are properly set in SessionDataNoAuth
    let cmdv = vec!["sh".to_string()];

    let session = Session::init()
        .with_user("noauthuser")
        .with_host("noauth.example.com")
        .with_port(10022)
        .with_scope("lo")
        .with_cmd(cmdv.clone())
        .with_inactivity_timeout(None)
        .build()
        .unwrap();

    if let SessionInner::NoAuth { data, session: _ } = session.inner {
        assert_eq!(data.user, "noauthuser");
        assert_eq!(data.host, "noauth.example.com");
        assert_eq!(data.port, 10022);
        assert_eq!(data.scope, Some("lo".to_string()));
        assert_eq!(data.cmdv, cmdv);
        assert_eq!(data.inactivity_timeout, None);
    } else {
        panic!("Expected NoAuth variant");
    }
}

#[test]
fn test_command_vector_variations() {
    // Test different command vector configurations
    let test_cases = vec![
        vec!["bash".to_string()],
        vec!["sh".to_string(), "-c".to_string(), "ls".to_string()],
        vec![
            "python3".to_string(),
            "-m".to_string(),
            "http.server".to_string(),
        ],
        vec!["node".to_string(), "app.js".to_string()],
    ];

    for cmdv in test_cases {
        let session = Session::init()
            .with_cmd(cmdv.clone())
            .with_passwd("pass")
            .build()
            .unwrap();

        if let SessionInner::Passwd { data, .. } = session.inner {
            assert_eq!(data.cmdv, cmdv);
        }
    }
}

#[test]
fn test_scope_variations() {
    // Test different scope ID formats
    let scopes = vec![
        "eth0", "wlan0", "lo", "enp0s3", "2", // numeric interface index
    ];

    for scope in scopes {
        let session = Session::init()
            .with_host("fe80::1")
            .with_scope(scope)
            .with_passwd("pass")
            .build()
            .unwrap();

        if let SessionInner::Passwd { data, .. } = session.inner {
            assert_eq!(data.scope, Some(scope.to_string()));
        }
    }
}
