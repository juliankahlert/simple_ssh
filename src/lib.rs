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
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Error;
use anyhow::Result;
use russh::keys::*;
use russh::*;
use tokio::io::{AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::client::Msg;
use tokio::fs::File;

use log::info;

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
        self.inner.system(None, true, true).await
    }

    pub async fn exec(&mut self, command: &Vec<String>) -> Result<u32> {
        self.inner.system(Some(command), false, false).await
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
    pub fn build(self) -> Result<Session> {
        if let Some(key) = self.key {
            return Ok(Session {
                inner: SessionInner::PubKey {
                    session: None,
                    data: SessionDataPubKey {
                        user: self.user.to_string(),
                        host: self.host.to_string(),
                        cmdv: self.cmdv,
                        port: self.port,
                        cert: self.cert,
                        key,
                    },
                },
            });
        } else if let Some(passwd) = self.passwd {
            return Ok(Session {
                inner: SessionInner::Passwd {
                    session: None,
                    data: SessionDataPasswd {
                        user: self.user.to_string(),
                        host: self.host.to_string(),
                        cmdv: self.cmdv,
                        port: self.port,
                        passwd,
                    },
                },
            });
        } else {
            return Ok(Session {
                inner: SessionInner::NoAuth {
                    session: None,
                    data: SessionDataNoAuth {
                        user: self.user.to_string(),
                        host: self.host.to_string(),
                        cmdv: self.cmdv,
                        port: self.port,
                    },
                },
            });
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
}

#[derive(Clone)]
struct SessionDataPubKey {
    cert: Option<PathBuf>,
    cmdv: Vec<String>,
    user: String,
    host: String,
    key: PathBuf,
    port: u16,
}

#[derive(Clone)]
struct SessionDataNoAuth {
    cmdv: Vec<String>,
    user: String,
    host: String,
    port: u16,
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
            } => {
                return self.connect_passwd().await;
            }
            Self::PubKey {
                data: _,
                session: _,
            } => {
                return self.connect_key().await;
            }
            Self::NoAuth {
                data: _,
                session: _,
            } => {
                return self.connect_noauth().await;
            }
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

    async fn system(&mut self, command: Option<&Vec<String>>, err: bool, out: bool) -> Result<u32> {
        let cmd = if let Some(c) = command {
            c.into_iter()
                .map(|x| shell_escape::escape(x.into())) // arguments are escaped manually since the SSH protocol doesn't support quoting
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            self.get_command()
        };

        if let Some(session) = self.get_session() {
            return system(session, &cmd, err, out).await;
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

        cmd.into_iter()
            .map(|x| shell_escape::escape(x.into())) // arguments are escaped manually since the SSH protocol doesn't support quoting
            .collect::<Vec<_>>()
            .join(" ")
    }

    async fn connect_noauth(self) -> Result<Self> {
        if let Self::NoAuth { data, session: _ } = self {
            let config = client::Config {
                inactivity_timeout: Some(Duration::from_secs(5)),
                ..<_>::default()
            };
            let config = Arc::new(config);
            let sh = Client {};
            let addrs = (data.host.clone(), data.port);
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
                inactivity_timeout: Some(Duration::from_secs(5)),
                ..<_>::default()
            };
            let config = Arc::new(config);
            let sh = Client {};
            let addrs = (data.host.clone(), data.port);
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
                data: data,
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
                inactivity_timeout: Some(Duration::from_secs(5)),
                ..<_>::default()
            };

            let config = Arc::new(config);
            let sh = Client {};
            let addrs = (data.host.clone(), data.port);
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
                data: data,
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
            Some(msg) = channel.wait() => {
                match msg {
                    // Write data to the terminal
                    ChannelMsg::Data { ref data } => {
                        stdout.write_all(data).await?;
                        stdout.flush().await?;
                    }
                    // The command has returned an exit code
                    ChannelMsg::ExitStatus { exit_status } => {
                        code = exit_status;
                        if !stdin_closed {
                            channel.eof().await?;
                        }
                        break;
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
            Some(ChannelMsg::ExtendedData { ref data, ext }) if ext == 1 => {
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

        let data = wait_for_data(&mut self.channel).await?;
        if data[0] != 0 {
            return Err(Error::msg(format!("SCP start failed: {:?}", data)));
        }

        let writer = Box::pin(self.channel.make_writer());

        Ok(SCPStateTxStart {
            channel: self.channel,
            writer,
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

use std::pin::Pin;
struct SCPStateTxStart {
    channel: Channel<Msg>,
    writer: Pin<Box<dyn AsyncWrite + Send>>,
}

impl SCPStateTxStart {
    async fn write_metadata(mut self, file_size: u64, file_name: &str) -> Result<SCPStateTxData> {
        let metadata_msg = format!("C0644 {} {}\n", file_size, file_name);
        self.writer.write_all(metadata_msg.as_bytes()).await?;
        self.writer.flush().await?;

        let data = wait_for_data(&mut self.channel).await?;
        if data[0] != 0 {
            return Err(Error::msg(format!("SCP confirmation failed: {:?}", data)));
        }

        Ok(SCPStateTxData {
            channel: self.channel,
            writer: self.writer,
        })
    }
}

struct SCPStateTxData {
    channel: Channel<Msg>,
    writer: Pin<Box<dyn AsyncWrite + Send>>,
}

impl SCPStateTxData {
    async fn write_data(&mut self, buf: &[u8]) -> Result<()> {
        self.writer.write_all(buf).await?;
        self.writer.flush().await?;

        Ok(())
    }

    async fn eof(mut self) -> Result<SCPStateEOF> {
        self.writer.write_all(b"\0").await?;
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
    let file_name = std::path::Path::new(local_path)
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Invalid file name"))?
        .to_string_lossy();

    let mut state = state.write_metadata(file_size, &file_name).await?;

    // Send the file contents in 16KB chunks
    let mut buffer = [0u8; 16 * 1024]; // 16KB buffer
    let mut reader = file;

    loop {
        let bytes_read = reader.read(&mut buffer).await?;
        if bytes_read == 0 {
            break; // EOF
        }

        state.write_data(&buffer[..bytes_read]).await?;
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
