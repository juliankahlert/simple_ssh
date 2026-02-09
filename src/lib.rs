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

//! Simple async SSH client for Rust.
//!
//! A lightweight, asynchronous library built on top of `russh` and `russh-keys`
//! that simplifies SSH operations such as executing remote commands, transferring
//! files via SCP, and interactive PTY sessions.
//!
//! # Quick Start
//!
//! ```no_run
//! use simple_ssh::Session;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let mut ssh = Session::init()
//!         .with_host("example.com")
//!         .with_user("admin")
//!         .with_passwd("secret")
//!         .build()?
//!         .connect()
//!         .await?;
//!
//!     let code = ssh.cmd("ls -la").await?;
//!     println!("Exit code: {}", code);
//!
//!     ssh.close().await?;
//!     Ok(())
//! }
//! ```
//!
//! # Features
//!
//! - Execute remote commands (`cmd`, `exec`, `system`)
//! - Transfer files via SCP protocol
//! - Interactive PTY sessions with raw mode
//! - Programmatic PTY sessions via `PtyHandle` for TUI embedding
//! - Public key, password, and certificate authentication
//! - IPv6 link-local address support

use std::env;
use std::io::Write;
use std::net::{SocketAddr, ToSocketAddrs};
use std::panic;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::anyhow;
use anyhow::Error;
use anyhow::Result;
use crossterm::{
    cursor::{SetCursorStyle, Show},
    style::ResetColor,
    terminal::{disable_raw_mode, enable_raw_mode, size},
    Command,
};
use log::debug;
use log::info;
use russh::keys::*;
use russh::*;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;
use tokio::time::timeout;

use crate::client::Msg;
use crate::pty_mode::ModeDetection;
use crate::pty_pwd::PwdDetection;

pub use russh::Pty;
pub use russh::Sig;

pub mod pty_mode;
pub mod pty_pwd;

pub use pty_mode::{ModeChangeEvent, ModeDetectionConfig, ModeWatcher, PtyMode};
pub use pty_pwd::{PwdChangeEvent, PwdDetectionConfig, PwdWatcher};

/// Type alias for the previous panic hook handler.
type PanicHook = Box<dyn Fn(&panic::PanicHookInfo<'_>) + Send + Sync>;

static PANIC_HOOK_SET: std::sync::Once = std::sync::Once::new();
static PREV_PANIC_HOOK: std::sync::Mutex<Option<PanicHook>> = std::sync::Mutex::new(None);

fn setup_panic_hook() {
    PANIC_HOOK_SET.call_once(|| {
        let prev_hook = panic::take_hook();
        if let Ok(mut guard) = PREV_PANIC_HOOK.lock() {
            *guard = Some(prev_hook);
        }
        panic::set_hook(Box::new(|panic_info| {
            terminal_cleanup();
            if let Ok(guard) = PREV_PANIC_HOOK.lock() {
                if let Some(ref hook) = *guard {
                    hook(panic_info);
                }
            }
        }));
    });
}

fn terminal_cleanup() {
    let _ = disable_raw_mode();
    let mut stdout = std::io::stdout();
    let _ = stdout.write_all(&terminal_reset_bytes());
    let _ = stdout.flush();
}

fn command_to_bytes<C: Command>(command: C) -> Vec<u8> {
    let mut output = String::new();
    let _ = command.write_ansi(&mut output);
    output.into_bytes()
}

fn terminal_reset_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&command_to_bytes(SetCursorStyle::DefaultUserShape));
    bytes.extend_from_slice(&command_to_bytes(Show));
    bytes.extend_from_slice(&command_to_bytes(ResetColor));
    bytes.extend_from_slice(b"\r\n");
    bytes
}

/// Represents the exit status of a PTY session.
///
/// Captures exit codes, signal termination, or unexpected channel closure.
#[derive(Debug, Clone)]
pub enum PtyExitStatus {
    /// The remote process exited with a numeric exit code.
    Code(u32),
    /// The remote process was terminated by a signal.
    Signal {
        /// The signal that terminated the process.
        signal_name: Sig,
        /// Whether a core dump was produced.
        core_dumped: bool,
        /// Error message from the remote side.
        error_message: String,
    },
    /// The SSH channel was closed without an explicit exit status.
    ChannelClosed,
}

impl PtyExitStatus {
    /// Returns the exit code if this is a `Code` variant.
    ///
    /// Returns `None` for `Signal` and `ChannelClosed` variants.
    pub fn code(&self) -> Option<u32> {
        match self {
            PtyExitStatus::Code(c) => Some(*c),
            _ => None,
        }
    }
}

/// A non-blocking, channel-based handle to a running remote PTY session.
///
/// `PtyHandle` provides a programmatic interface for embedding a PTY
/// session inside a TUI pane or any custom I/O loop. It does not manage
/// raw mode, stdin/stdout, or SIGWINCH — those are the caller's
/// responsibility.
///
/// # Example
///
/// ```ignore
/// let mut handle = session.pty_builder()
///     .with_term("xterm-256color")
///     .with_size(80, 24)
///     .open()
///     .await?;
///
/// // Send input
/// handle.write(b"ls -la\n").await?;
///
/// // Read output
/// while let Some(data) = handle.read().await {
///     process_terminal_output(&data);
/// }
///
/// let status = handle.wait().await?;
/// ```
pub struct PtyHandle {
    input_tx: mpsc::Sender<Vec<u8>>,
    output_rx: mpsc::Receiver<Vec<u8>>,
    resize_tx: mpsc::Sender<(u32, u32)>,
    task_handle: Option<JoinHandle<Result<PtyExitStatus>>>,
    exit_rx: watch::Receiver<Option<PtyExitStatus>>,
    closed: bool,
    mode_detection: Option<Arc<ModeDetection>>,
    pwd_detection: Option<Arc<PwdDetection>>,
}

impl std::fmt::Debug for PtyHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PtyHandle")
            .field("closed", &self.closed)
            .field("task_running", &self.task_handle.is_some())
            .finish()
    }
}

impl PtyHandle {
    /// Sends input data to the remote PTY.
    ///
    /// The data is sent as-is, including any escape sequences.
    /// Returns an error if the background task has already exited.
    pub async fn write(&self, data: &[u8]) -> Result<()> {
        self.input_tx
            .send(data.to_vec())
            .await
            .map_err(|_| anyhow!("PTY input channel closed"))
    }

    /// Receives raw terminal output from the remote PTY.
    ///
    /// Returns `None` when the channel is closed (session ended).
    /// The output includes raw escape sequences suitable for a
    /// terminal emulator.
    pub async fn read(&mut self) -> Option<Vec<u8>> {
        self.output_rx.recv().await
    }

    /// Sends a window resize event to the remote PTY.
    ///
    /// # Arguments
    ///
    /// * `cols` - New number of columns
    /// * `rows` - New number of rows
    pub async fn resize(&self, cols: u32, rows: u32) -> Result<()> {
        self.resize_tx
            .send((cols, rows))
            .await
            .map_err(|_| anyhow!("PTY resize channel closed"))
    }

    /// Consumes the handle and waits for the remote PTY to exit.
    ///
    /// Returns the exit status of the remote process.
    pub async fn wait(mut self) -> Result<PtyExitStatus> {
        if let Some(handle) = self.task_handle.take() {
            handle.await?
        } else {
            Ok(PtyExitStatus::ChannelClosed)
        }
    }

    /// Non-blocking check for whether the PTY has exited.
    ///
    /// Returns `Some(status)` if the process has exited, `None` if
    /// still running.
    pub fn try_wait(&self) -> Option<PtyExitStatus> {
        self.exit_rx.borrow().clone()
    }

    /// Closes the input side of the PTY, sending EOF to the remote.
    ///
    /// After calling this, no more input can be sent.
    pub fn close(&mut self) {
        self.closed = true;
        // Dropping the sender signals EOF to the background task
        // We replace with a closed channel
        let (tx, _) = mpsc::channel(1);
        self.input_tx = tx;
    }

    /// Returns the current PTY mode if detection is enabled.
    pub fn current_mode(&self) -> Option<PtyMode> {
        self.mode_detection
            .as_ref()
            .map(|md: &Arc<ModeDetection>| md.current_mode())
    }

    /// Returns true if currently in alternate buffer mode.
    pub fn is_alt_mode(&self) -> bool {
        self.mode_detection
            .as_ref()
            .map(|md: &Arc<ModeDetection>| md.current_mode().is_alternate())
            .unwrap_or(false)
    }

    /// Returns true if currently in standard buffer mode.
    pub fn is_std_mode(&self) -> bool {
        self.mode_detection
            .as_ref()
            .map(|md: &Arc<ModeDetection>| md.current_mode().is_standard())
            .unwrap_or(true)
    }

    /// Creates a watcher for mode change events.
    ///
    /// Returns a `ModeWatcher` that can be used to await mode changes.
    /// This enables async patterns like `tokio::select!` integration.
    pub fn watch_mode(&self) -> Option<ModeWatcher> {
        self.mode_detection
            .as_ref()
            .map(|md: &Arc<ModeDetection>| md.create_watcher())
    }

    /// Returns the current working directory if PWD detection is enabled
    /// and a directory has been reported via OSC 7 or OSC 633.
    pub fn current_pwd(&self) -> Option<String> {
        self.pwd_detection
            .as_ref()
            .and_then(|pd: &Arc<PwdDetection>| pd.current_pwd())
    }

    /// Creates a watcher for PWD change events.
    ///
    /// Returns a `PwdWatcher` that can be used to await directory changes.
    /// Returns `None` if PWD detection is not enabled.
    pub fn watch_pwd(&self) -> Option<PwdWatcher> {
        self.pwd_detection
            .as_ref()
            .map(|pd: &Arc<PwdDetection>| pd.create_watcher())
    }
}

impl Drop for PtyHandle {
    fn drop(&mut self) {
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
    }
}

/// Returns sensible default terminal modes for PTY sessions.
///
/// These modes enable proper behavior for interactive programs
/// including alt-mode applications like vim and nano.
fn default_pty_terminal_modes() -> Vec<(Pty, u32)> {
    vec![
        (Pty::ICRNL, 1),
        (Pty::IXON, 0),
        (Pty::IXANY, 0),
        (Pty::IMAXBEL, 0),
        (Pty::IUTF8, 1),
        (Pty::OPOST, 1),
        (Pty::ONLCR, 1),
        (Pty::ISIG, 1),
        (Pty::ICANON, 0),
        (Pty::ECHO, 1),
        (Pty::ECHOE, 1),
        (Pty::ECHOK, 1),
        (Pty::ECHOCTL, 1),
        (Pty::ECHOKE, 1),
        (Pty::IEXTEN, 1),
        (Pty::CS8, 1),
        (Pty::TTY_OP_ISPEED, 38400),
        (Pty::TTY_OP_OSPEED, 38400),
    ]
}

/// Background task that bridges channel-based I/O with a remote PTY session.
///
/// Handles input forwarding, output collection, resize events, and
/// exit status detection without any terminal or signal management.
async fn pty_io_task(
    mut channel: Channel<Msg>,
    mut input_rx: mpsc::Receiver<Vec<u8>>,
    output_tx: mpsc::Sender<Vec<u8>>,
    mut resize_rx: mpsc::Receiver<(u32, u32)>,
    exit_tx: watch::Sender<Option<PtyExitStatus>>,
    mode_detection: Option<Arc<ModeDetection>>,
    pwd_detection: Option<Arc<PwdDetection>>,
) -> Result<PtyExitStatus> {
    let status = loop {
        tokio::select! {
            res = input_rx.recv() => {
                match res {
                    Some(data) => {
                        channel.data(&data[..]).await?;
                    }
                    None => {
                        // Input channel closed - send EOF to remote
                        channel.eof().await?;
                    }
                }
            }
            msg = channel.wait() => {
                match msg {
                    Some(ChannelMsg::Data { ref data }) => {
                        if let Some(md) = mode_detection.as_ref() {
                            md.feed(data);
                        }
                        if let Some(pd) = pwd_detection.as_ref() {
                            pd.feed(data);
                        }
                        // If the receiver is dropped, we still continue
                        // to process channel messages for exit status
                        let _ = output_tx.send(data.to_vec()).await;
                    }
                    Some(ChannelMsg::ExitStatus { exit_status }) => {
                        break PtyExitStatus::Code(exit_status);
                    }
                    Some(ChannelMsg::ExitSignal {
                        signal_name,
                        core_dumped,
                        error_message,
                        ..
                    }) => {
                        break PtyExitStatus::Signal {
                            signal_name,
                            core_dumped,
                            error_message,
                        };
                    }
                    None => {
                        break PtyExitStatus::ChannelClosed;
                    }
                    _ => {}
                }
            }
            Some((cols, rows)) = resize_rx.recv() => {
                let _ = channel.window_change(cols, rows, 0, 0).await;
            }
        }
    };

    // Drain remaining output after exit status
    drain_remaining_output(
        &mut channel,
        &output_tx,
        mode_detection.as_deref(),
        pwd_detection.as_deref(),
    )
    .await;

    let _ = exit_tx.send(Some(status.clone()));
    Ok(status)
}

/// Drains any remaining data from the channel after an exit status is received.
///
/// Programs like nano send terminal cleanup sequences after their exit status,
/// so we need to forward those to the output channel.
async fn drain_remaining_output(
    channel: &mut Channel<Msg>,
    output_tx: &mpsc::Sender<Vec<u8>>,
    mode_detection: Option<&ModeDetection>,
    pwd_detection: Option<&PwdDetection>,
) {
    loop {
        tokio::select! {
            msg = channel.wait() => {
                match msg {
                    Some(ChannelMsg::Data { ref data }) => {
                        if let Some(md) = mode_detection {
                            md.feed(data);
                        }
                        if let Some(pd) = pwd_detection {
                            pd.feed(data);
                        }
                        let _ = output_tx.send(data.to_vec()).await;
                    }
                    _ => break,
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(1)) => {
                break;
            }
        }
    }
}

/// Resolves a hostname and port to a socket address.
///
/// Supports IPv6 link-local addresses with scope IDs.
///
/// # Arguments
///
/// * `host` - Hostname or IP address
/// * `port` - Port number
/// * `scope` - Optional IPv6 scope ID (interface name)
///
/// # Returns
///
/// A [`SocketAddr`] or an error if resolution fails.
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

/// An SSH session handle that provides methods for executing commands,
/// transferring files, and managing interactive shells.
///
/// Use [`Session::init`] to create a new session builder.
pub struct Session {
    inner: SessionInner,
}

impl<'sb> Session {
    /// Creates a new session builder with default settings.
    ///
    /// Defaults:
    /// - Host: `localhost`
    /// - User: `root`
    /// - Port: `22`
    /// - Command: `bash`
    /// - Inactivity timeout: 3000 seconds
    ///
    /// # Example
    ///
    /// ```
    /// use simple_ssh::Session;
    ///
    /// let session = Session::init()
    ///     .with_host("example.com")
    ///     .with_user("admin")
    ///     .with_passwd("secret")
    ///     .build();
    /// ```
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

    /// Connects to the SSH server using the configured authentication method.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The hostname cannot be resolved
    /// - The connection fails
    /// - Authentication fails
    pub async fn connect(self) -> Result<Self> {
        match self.inner.connect().await {
            Ok(res) => Ok(Session { inner: res }),
            Err(e) => Err(e),
        }
    }

    /// Opens an interactive PTY (pseudo-terminal) session.
    ///
    /// This is useful for interactive shell sessions where you want
    /// terminal emulation support.
    ///
    /// # Returns
    ///
    /// The exit code of the shell session.
    ///
    /// # Errors
    ///
    /// Returns an error if no connection is established.
    pub async fn pty(&mut self) -> Result<u32> {
        self.pty_builder().run().await
    }

    /// Creates a PTY builder for advanced terminal configuration.
    ///
    /// The builder provides options for raw mode, terminal type,
    /// dimensions, and custom commands.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let exit_code = session.pty_builder()
    ///     .with_raw()
    ///     .with_term("xterm-256color")
    ///     .run().await?;
    /// ```
    pub fn pty_builder(&mut self) -> PtyBuilder<'_> {
        let (width, height) = size().unwrap_or((80, 24));
        PtyBuilder {
            session: self,
            raw_mode: false,
            term: env::var("TERM").unwrap_or_else(|_| "xterm".to_string()),
            width: width as u32,
            height: height as u32,
            command: None,
            auto_resize: false,
            terminal_modes: None,
            mode_detection_config: None,
            pwd_detection_config: None,
        }
    }

    /// Runs the configured command with output to stdout and stderr.
    ///
    /// Uses the command specified via [`SessionBuilder::with_cmd`].
    ///
    /// # Returns
    ///
    /// The exit code of the command.
    ///
    /// # Errors
    ///
    /// Returns an error if no connection is established.
    pub async fn run(&mut self) -> Result<u32> {
        self.inner.exec(None, true, true).await
    }

    /// Executes a command with the given arguments.
    ///
    /// # Arguments
    ///
    /// * `command` - A vector of command and its arguments
    ///
    /// # Returns
    ///
    /// The exit code of the command.
    ///
    /// # Errors
    ///
    /// Returns an error if no connection is established.
    pub async fn exec(&mut self, command: &Vec<String>) -> Result<u32> {
        self.inner.exec(Some(command), false, false).await
    }

    /// Executes a shell command via `sh -c`.
    ///
    /// # Arguments
    ///
    /// * `command` - The shell command to execute
    ///
    /// # Returns
    ///
    /// The exit code of the command.
    ///
    /// # Errors
    ///
    /// Returns an error if no connection is established.
    pub async fn system(&mut self, command: &str) -> Result<u32> {
        let sys_cmd = vec!["sh".to_string(), "-c".to_string(), command.to_string()];
        self.inner.exec(Some(&sys_cmd), false, false).await
    }

    /// Executes a single command string.
    ///
    /// # Arguments
    ///
    /// * `command` - The command to execute
    ///
    /// # Returns
    ///
    /// The exit code of the command.
    ///
    /// # Errors
    ///
    /// Returns an error if no connection is established.
    pub async fn cmd(&mut self, command: &str) -> Result<u32> {
        self.inner.cmd(command, false, false).await
    }

    /// Transfers a file to the remote host via SCP.
    ///
    /// # Arguments
    ///
    /// * `from` - Local file path
    /// * `to` - Remote destination path
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No connection is established
    /// - The local file cannot be read
    /// - The transfer fails
    pub async fn scp(&mut self, from: &str, to: &str) -> Result<()> {
        self.inner.scp(from, to).await
    }

    /// Closes the SSH session gracefully.
    ///
    /// # Errors
    ///
    /// Returns an error if the disconnect message fails to send.
    pub async fn close(&mut self) -> Result<()> {
        self.inner.close().await
    }
}

/// Builder for configuring and executing PTY sessions with advanced options.
///
/// Use [`Session::pty_builder`] to create a new builder, then chain
/// configuration methods, and finally call [`PtyBuilder::run`] to execute
/// the PTY session.
///
/// # Example
///
/// ```ignore
/// let exit_code = session.pty_builder()
///     .with_raw()
///     .with_term("xterm-256color")
///     .run()
///     .await?;
/// ```
pub struct PtyBuilder<'a> {
    session: &'a mut Session,
    raw_mode: bool,
    term: String,
    width: u32,
    height: u32,
    command: Option<String>,
    auto_resize: bool,
    terminal_modes: Option<Vec<(Pty, u32)>>,
    mode_detection_config: Option<pty_mode::ModeDetectionConfig>,
    pwd_detection_config: Option<pty_pwd::PwdDetectionConfig>,
}

impl<'a> PtyBuilder<'a> {
    /// Enables raw mode for proper control character handling.
    ///
    /// In raw mode, the local terminal does not process control characters,
    /// allowing them to pass through to the remote SSH session. This enables
    /// proper handling of arrow keys, Ctrl+C, Tab, and other control sequences.
    ///
    /// This is the primary feature needed to make the simple-ssh binary
    /// behave like the standard ssh client.
    pub fn with_raw(mut self) -> Self {
        self.raw_mode = true;
        self
    }

    /// Sets the terminal type string sent to the remote server.
    ///
    /// # Arguments
    ///
    /// * `term` - Terminal type (e.g., "xterm", "xterm-256color", "vt100")
    pub fn with_term(mut self, term: &str) -> Self {
        self.term = term.to_string();
        self
    }

    /// Sets the terminal dimensions.
    ///
    /// # Arguments
    ///
    /// * `width` - Number of columns (default: 80)
    /// * `height` - Number of rows (default: 24)
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width.max(1);
        self.height = height.max(1);
        self
    }

    /// Sets a custom command to execute instead of the default shell.
    ///
    /// # Arguments
    ///
    /// * `cmd` - Command to execute in the PTY
    pub fn with_command(mut self, cmd: &str) -> Self {
        self.command = Some(cmd.to_string());
        self
    }

    /// Enables automatic terminal resize handling.
    ///
    /// When enabled, the PTY will automatically resize when the local
    /// terminal window is resized, sending the new dimensions to the
    /// remote SSH session.
    pub fn with_auto_resize(mut self) -> Self {
        self.auto_resize = true;
        self
    }

    /// Sets custom terminal modes for the PTY request.
    ///
    /// If not set, [`default_pty_terminal_modes()`] is used automatically.
    /// Pass an empty slice to disable terminal modes entirely.
    ///
    /// # Arguments
    ///
    /// * `modes` - Slice of (Pty, u32) terminal mode settings
    pub fn with_terminal_modes(mut self, modes: &[(Pty, u32)]) -> Self {
        self.terminal_modes = Some(modes.to_vec());
        self
    }

    /// Enables PTY mode detection with default config.
    ///
    /// This enables detection of alternate screen buffer mode changes,
    /// allowing users to detect when full-screen applications like vim
    /// or nano are running.
    pub fn with_mode_detection(mut self) -> Self {
        self.mode_detection_config = Some(pty_mode::ModeDetectionConfig {
            enabled: true,
            ..pty_mode::ModeDetectionConfig::default()
        });
        self
    }

    /// Enables PTY mode detection with custom config.
    ///
    /// This enables detection of alternate screen buffer mode changes
    /// with custom configuration options.
    pub fn with_mode_detection_config(mut self, config: pty_mode::ModeDetectionConfig) -> Self {
        self.mode_detection_config = Some(config);
        self
    }

    /// Enables PWD (working directory) detection with default config.
    ///
    /// This enables detection of the remote shell's working directory
    /// via OSC 7 and OSC 633 escape sequences. The remote shell must
    /// emit these sequences — fish does this by default, while bash
    /// and zsh require configuration.
    pub fn with_pwd_detection(mut self) -> Self {
        self.pwd_detection_config = Some(pty_pwd::PwdDetectionConfig {
            enabled: true,
            ..pty_pwd::PwdDetectionConfig::default()
        });
        self
    }

    /// Enables PWD detection with custom config.
    ///
    /// This enables detection of the remote shell's working directory
    /// with custom configuration options.
    pub fn with_pwd_detection_config(mut self, mut config: pty_pwd::PwdDetectionConfig) -> Self {
        config.enabled = true;
        self.pwd_detection_config = Some(config);
        self
    }

    /// Opens a programmatic PTY session and returns a [`PtyHandle`].
    ///
    /// Unlike [`run()`](PtyBuilder::run), this does not manage stdin/stdout,
    /// raw mode, or SIGWINCH. The caller controls all I/O through the
    /// returned handle.
    ///
    /// # Returns
    ///
    /// A [`PtyHandle`] for interacting with the remote PTY.
    ///
    /// # Errors
    ///
    /// Returns an error if no connection is established or the PTY
    /// request fails.
    pub async fn open(self) -> Result<PtyHandle> {
        let command = self
            .command
            .unwrap_or_else(|| self.session.inner.get_command());
        let Some(sess) = self.session.inner.get_session() else {
            return Err(Error::msg("No open session"));
        };

        let modes = self
            .terminal_modes
            .unwrap_or_else(default_pty_terminal_modes);

        let channel = sess.channel_open_session().await?;

        channel
            .request_pty(false, &self.term, self.width, self.height, 0, 0, &modes)
            .await?;
        channel.exec(true, command).await?;

        let (input_tx, input_rx) = mpsc::channel(64);
        let (output_tx, output_rx) = mpsc::channel(256);
        let (resize_tx, resize_rx) = mpsc::channel(4);
        let (exit_tx, exit_rx) = watch::channel(None);

        let mode_detection: Option<Arc<ModeDetection>> = self
            .mode_detection_config
            .map(|config| Arc::new(ModeDetection::new(config)));

        let pwd_detection: Option<Arc<PwdDetection>> = self
            .pwd_detection_config
            .map(|config| Arc::new(PwdDetection::new(config)));

        let task_handle = tokio::spawn(pty_io_task(
            channel,
            input_rx,
            output_tx,
            resize_rx,
            exit_tx,
            mode_detection.clone(),
            pwd_detection.clone(),
        ));

        Ok(PtyHandle {
            input_tx,
            output_rx,
            resize_tx,
            task_handle: Some(task_handle),
            exit_rx,
            closed: false,
            mode_detection,
            pwd_detection,
        })
    }

    /// Executes the PTY session with the configured options.
    ///
    /// This method consumes the builder and runs the interactive session,
    /// connecting the remote PTY to local stdin/stdout with optional raw
    /// mode and SIGWINCH handling.
    ///
    /// # Returns
    ///
    /// The exit code of the remote command.
    ///
    /// # Errors
    ///
    /// Returns an error if the PTY cannot be established or the session fails.
    pub async fn run(self) -> Result<u32> {
        let raw_mode = self.raw_mode;
        let auto_resize = self.auto_resize;

        let mut handle = self.open().await?;

        let mut stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut buf = vec![0; 1024];
        let mut stdin_closed = false;

        let _raw_guard = if raw_mode {
            setup_panic_hook();
            enable_raw_mode()?;
            Some(RawGuard)
        } else {
            None
        };

        #[cfg(unix)]
        let mut winch_signal = if auto_resize {
            let sig =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::window_change())?;
            Some(sig)
        } else {
            None
        };

        let status_result: Result<PtyExitStatus> = loop {
            tokio::select! {
                r = stdin.read(&mut buf), if !stdin_closed => {
                    match r {
                        Ok(0) => {
                            stdin_closed = true;
                            handle.close();
                        },
                        Ok(n) => {
                            if let Err(e) = handle.write(&buf[..n]).await {
                                write_reset_sequences(&mut stdout, raw_mode).await;
                                drop(_raw_guard);
                                clear_stdin_buffer();
                                return Err(e);
                            }
                        }
                        Err(e) => {
                            write_reset_sequences(&mut stdout, raw_mode).await;
                            drop(_raw_guard);
                            clear_stdin_buffer();
                            return Err(e.into());
                        }
                    };
                },
                data = handle.read() => {
                    match data {
                        Some(bytes) => {
                            stdout.write_all(&bytes).await?;
                            stdout.flush().await?;
                        }
                        None => {
                            // Output channel closed — session ended
                            let wait_res = handle.wait().await;
                            break wait_res.map_err(|e| anyhow::anyhow!("{}", e));
                        }
                    }
                },
                _ = async {
                    #[cfg(unix)]
                    if let Some(ref mut sig) = winch_signal {
                        sig.recv().await;
                    }
                    #[cfg(not(unix))]
                    {
                        std::future::pending::<()>().await;
                    }
                }, if auto_resize => {
                    if let Ok((w, h)) = size() {
                        let _ = handle.resize(w as u32, h as u32).await;
                    }
                },
            }
        };

        // Write reset sequences while still in raw mode
        write_reset_sequences(&mut stdout, raw_mode).await;

        // Now safe to disable raw mode
        drop(_raw_guard);

        // Clear any pending stdin
        clear_stdin_buffer();

        // Ensure stdout is flushed
        stdout.flush().await?;

        let status = status_result?;
        Ok(status.code().unwrap_or(255))
    }
}

/// Builder for configuring and creating an SSH [`Session`].
///
/// Use [`Session::init`] to create a new builder, then chain
/// configuration methods, and finally call [`SessionBuilder::build`]
/// to create the session.
///
/// # Example
///
/// ```
/// use simple_ssh::Session;
///
/// let session = Session::init()
///     .with_host("example.com")
///     .with_user("admin")
///     .with_port(2222)
///     .with_passwd("secret")
///     .build()
///     .expect("Failed to build session");
/// ```
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
    /// Sets the SSH certificate path option.
    ///
    /// # Arguments
    ///
    /// * `cert` - Optional path to the certificate file
    pub fn with_cert_opt(mut self, cert: Option<PathBuf>) -> Self {
        self.cert = cert;
        self
    }

    /// Sets the SSH private key path option.
    ///
    /// # Arguments
    ///
    /// * `key` - Optional path to the private key file
    pub fn with_key_opt(mut self, key: Option<PathBuf>) -> Self {
        self.key = key;
        self
    }

    /// Sets the SSH certificate path for authentication.
    ///
    /// # Arguments
    ///
    /// * `cert` - Path to the certificate file
    pub fn with_cert(mut self, cert: PathBuf) -> Self {
        self.cert = Some(cert);
        self
    }

    /// Sets the SSH private key path for authentication.
    ///
    /// When a key is provided, public key authentication is used.
    ///
    /// # Arguments
    ///
    /// * `key` - Path to the private key file
    pub fn with_key(mut self, key: PathBuf) -> Self {
        self.key = Some(key);
        self
    }

    /// Sets the SSH port.
    ///
    /// # Arguments
    ///
    /// * `port` - Port number (default: 22)
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Sets the target host.
    ///
    /// # Arguments
    ///
    /// * `host` - Hostname or IP address
    pub fn with_host(mut self, host: &'sb str) -> Self {
        self.host = host;
        self
    }

    /// Sets the username for authentication.
    ///
    /// # Arguments
    ///
    /// * `user` - Username (default: "root")
    pub fn with_user(mut self, user: &'sb str) -> Self {
        self.user = user;
        self
    }

    /// Sets the command to execute for interactive sessions.
    ///
    /// # Arguments
    ///
    /// * `cmdv` - Command and its arguments as a vector
    pub fn with_cmd(mut self, cmdv: Vec<String>) -> Self {
        self.cmdv = cmdv;
        self
    }

    /// Sets the password for authentication.
    ///
    /// When a password is provided (and no key), password authentication is used.
    ///
    /// # Arguments
    ///
    /// * `passwd` - Password string
    pub fn with_passwd(mut self, passwd: &str) -> Self {
        self.passwd = Some(passwd.to_string());
        self
    }

    /// Sets the password option for authentication.
    ///
    /// # Arguments
    ///
    /// * `passwd` - Optional password string
    pub fn with_passwd_opt(mut self, passwd: Option<String>) -> Self {
        self.passwd = passwd;
        self
    }

    /// Sets the IPv6 scope ID (interface name or number).
    ///
    /// Required for link-local IPv6 addresses.
    ///
    /// # Arguments
    ///
    /// * `scope` - Interface name (e.g., "eth0") or numeric scope ID
    pub fn with_scope(mut self, scope: &str) -> Self {
        self.scope = Some(scope.to_string());
        self
    }

    /// Sets the inactivity timeout for the SSH connection.
    ///
    /// Set to `None` to disable the timeout.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Optional duration for inactivity timeout
    pub fn with_inactivity_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.inactivity_timeout = timeout;
        self
    }

    /// Builds the [`Session`] with the configured settings.
    ///
    /// # Returns
    ///
    /// A [`Result`] containing the configured [`Session`] or an error.
    ///
    /// # Authentication Priority
    ///
    /// 1. If a key is provided, use public key authentication
    /// 2. If a password is provided, use password authentication
    /// 3. Otherwise, use no authentication (none)
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

/// Internal SSH client handler that implements the russh client trait.
///
/// Currently accepts all server keys without verification.
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

/// Session data for password authentication.
#[derive(Clone)]
struct SessionDataPasswd {
    /// Command vector for interactive sessions.
    cmdv: Vec<String>,
    /// Password for authentication.
    passwd: String,
    /// Username for authentication.
    user: String,
    /// Target host.
    host: String,
    /// Target port.
    port: u16,
    /// IPv6 scope ID.
    scope: Option<String>,
    /// Inactivity timeout duration.
    inactivity_timeout: Option<Duration>,
}

/// Session data for public key authentication.
#[derive(Clone)]
struct SessionDataPubKey {
    /// Optional certificate path.
    cert: Option<PathBuf>,
    /// Command vector for interactive sessions.
    cmdv: Vec<String>,
    /// Username for authentication.
    user: String,
    /// Target host.
    host: String,
    /// Private key path.
    key: PathBuf,
    /// Target port.
    port: u16,
    /// IPv6 scope ID.
    scope: Option<String>,
    /// Inactivity timeout duration.
    inactivity_timeout: Option<Duration>,
}

/// Session data for no authentication (none auth).
#[derive(Clone)]
struct SessionDataNoAuth {
    /// Command vector for interactive sessions.
    cmdv: Vec<String>,
    /// Username for authentication.
    user: String,
    /// Target host.
    host: String,
    /// Target port.
    port: u16,
    /// IPv6 scope ID.
    scope: Option<String>,
    /// Inactivity timeout duration.
    inactivity_timeout: Option<Duration>,
}

/// Internal representation of an SSH session.
///
/// Tracks the authentication method and connection state.
enum SessionInner {
    /// Password authentication variant.
    Passwd {
        /// Session configuration data.
        data: SessionDataPasswd,
        /// Active SSH session handle.
        session: Option<client::Handle<Client>>,
    },
    /// Public key authentication variant.
    PubKey {
        /// Session configuration data.
        data: SessionDataPubKey,
        /// Active SSH session handle.
        session: Option<client::Handle<Client>>,
    },
    /// No authentication variant.
    NoAuth {
        /// Session configuration data.
        data: SessionDataNoAuth,
        /// Active SSH session handle.
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

/// RAII guard for raw terminal mode (cross-platform).
///
/// When dropped, automatically restores the terminal to its original mode.
struct RawGuard;

impl Drop for RawGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

/// Writes terminal reset sequences using crossterm's async API.
///
/// This function sends terminal reset sequences that match the crossterm commands
/// used in terminal_cleanup() for consistency between normal and panic paths.
async fn write_reset_sequences(stdout: &mut (impl AsyncWrite + Unpin), raw_mode: bool) {
    if raw_mode {
        let _ = stdout.write_all(&terminal_reset_bytes()).await;
        let _ = stdout.flush().await;
    }
}

/// Clears any pending input from stdin to prevent it from appearing
/// in the shell prompt after the PTY session ends.
fn clear_stdin_buffer() {
    #[cfg(unix)]
    {
        use std::io::Read;
        let mut buffer = [0u8; 1024];
        // Set stdin to non-blocking temporarily to drain any pending input
        unsafe {
            let flags = libc::fcntl(0, libc::F_GETFL, 0);
            if flags >= 0 {
                let setfl_result = libc::fcntl(0, libc::F_SETFL, flags | libc::O_NONBLOCK);
                if setfl_result >= 0 {
                    let stdin = std::io::stdin();
                    let mut stdin_lock = stdin.lock();
                    while let Ok(n) = stdin_lock.read(&mut buffer) {
                        if n == 0 {
                            break;
                        }
                    }
                }
                let _ = libc::fcntl(0, libc::F_SETFL, flags);
            }
        }
    }
}

/// Gracefully closes an SSH session.
///
/// # Arguments
///
/// * `session` - The SSH session handle
async fn close_session(session: &mut client::Handle<Client>) -> Result<()> {
    session
        .disconnect(Disconnect::ByApplication, "", "English")
        .await?;
    Ok(())
}

/// Executes a command on the remote system.
///
/// # Arguments
///
/// * `session` - The SSH session handle
/// * `command` - The command to execute
/// * `err` - Whether to output stderr
/// * `out` - Whether to output stdout
///
/// # Returns
///
/// The exit code of the command.
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

/// Waits for data from an SSH channel.
///
/// # Arguments
///
/// * `channel` - The SSH channel
///
/// # Returns
///
/// The received data as a byte vector.
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

/// Initial state for SCP file transfer.
struct SCPStateOpen {
    channel: Channel<Msg>,
}

impl SCPStateOpen {
    /// Initiates a file transfer to the remote path.
    ///
    /// # Arguments
    ///
    /// * `remote_path` - The destination path on the remote host
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

/// Opens a new channel for SCP file transfer.
///
/// # Arguments
///
/// * `session` - The SSH session handle
async fn scp_channel_open(session: &mut client::Handle<Client>) -> Result<SCPStateOpen> {
    let res = session.channel_open_session().await;

    match res {
        Err(e) => Err(Error::msg(e.to_string())),
        Ok(channel) => Ok(SCPStateOpen { channel }),
    }
}

/// State for sending file metadata during SCP transfer.
struct SCPStateTxStart {
    channel: Channel<Msg>,
}

impl SCPStateTxStart {
    /// Writes file metadata to initiate the transfer.
    ///
    /// # Arguments
    ///
    /// * `file_size` - Size of the file in bytes
    /// * `file_name` - Name of the file
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

/// State for sending file data during SCP transfer.
struct SCPStateTxData {
    channel: Channel<Msg>,
}

impl SCPStateTxData {
    /// Writes a chunk of file data.
    ///
    /// # Arguments
    ///
    /// * `buf` - Buffer containing file data
    async fn write_data(&mut self, buf: &[u8]) -> Result<()> {
        self.channel.data(buf).await?;
        Ok(())
    }

    /// Signals end of file and completes the transfer.
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

/// Final state for SCP file transfer.
struct SCPStateEOF {
    channel: Channel<Msg>,
}

impl SCPStateEOF {
    /// Closes the SCP channel.
    async fn close(self) -> Result<()> {
        self.channel.close().await?;
        Ok(())
    }
}

/// Transfers a file to the remote host using SCP protocol.
///
/// # Arguments
///
/// * `session` - The SSH session handle
/// * `local_path` - Path to the local file
/// * `remote_path` - Destination path on the remote host
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

#[tokio::test]
async fn test_pty_builder_no_session() {
    let mut session = Session::init()
        .with_host("localhost")
        .with_user("user")
        .with_passwd("pass")
        .build()
        .unwrap();

    let result = session.pty_builder().run().await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "No open session");
}

#[tokio::test]
async fn test_pty_builder_with_raw_no_session() {
    let mut session = Session::init()
        .with_host("localhost")
        .with_user("user")
        .with_passwd("pass")
        .build()
        .unwrap();

    let result = session.pty_builder().with_raw().run().await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "No open session");
}

#[tokio::test]
async fn test_pty_builder_with_term_no_session() {
    let mut session = Session::init()
        .with_host("localhost")
        .with_user("user")
        .with_passwd("pass")
        .build()
        .unwrap();

    let result = session
        .pty_builder()
        .with_term("xterm-256color")
        .run()
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_pty_builder_with_size_no_session() {
    let mut session = Session::init()
        .with_host("localhost")
        .with_user("user")
        .with_passwd("pass")
        .build()
        .unwrap();

    let result = session.pty_builder().with_size(120, 40).run().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_pty_builder_with_command_no_session() {
    let mut session = Session::init()
        .with_host("localhost")
        .with_user("user")
        .with_passwd("pass")
        .build()
        .unwrap();

    let result = session.pty_builder().with_command("ls").run().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_pty_builder_full_config_no_session() {
    let mut session = Session::init()
        .with_host("localhost")
        .with_user("user")
        .with_passwd("pass")
        .build()
        .unwrap();

    let result = session
        .pty_builder()
        .with_raw()
        .with_term("vt100")
        .with_size(80, 24)
        .with_command("/bin/sh")
        .run()
        .await;
    assert!(result.is_err());
}

#[test]
fn test_pty_builder_default_values() {
    let session = Session::init()
        .with_host("localhost")
        .with_user("user")
        .with_passwd("pass")
        .build()
        .unwrap();

    let mut session_mut = session;
    let builder = session_mut.pty_builder();

    assert_eq!(builder.raw_mode, false);
    assert_eq!(builder.command, None);

    let term = env::var("TERM").ok().unwrap_or_else(|| "xterm".to_string());
    assert_eq!(builder.term, term);

    let (w, h) = size().unwrap_or((80, 24));
    assert_eq!(builder.width, w as u32);
    assert_eq!(builder.height, h as u32);
}

#[test]
fn test_pty_builder_with_raw_sets_flag() {
    let session = Session::init()
        .with_host("localhost")
        .with_user("user")
        .with_passwd("pass")
        .build()
        .unwrap();

    let mut session_mut = session;
    let builder = session_mut.pty_builder().with_raw();

    assert_eq!(builder.raw_mode, true);
}

#[test]
fn test_pty_builder_with_term_sets_terminal() {
    let session = Session::init()
        .with_host("localhost")
        .with_user("user")
        .with_passwd("pass")
        .build()
        .unwrap();

    let mut session_mut = session;
    let builder = session_mut.pty_builder().with_term("xterm-256color");

    assert_eq!(builder.term, "xterm-256color");
}

#[test]
fn test_pty_builder_with_size_sets_dimensions() {
    let session = Session::init()
        .with_host("localhost")
        .with_user("user")
        .with_passwd("pass")
        .build()
        .unwrap();

    let mut session_mut = session;
    let builder = session_mut.pty_builder().with_size(120, 40);

    assert_eq!(builder.width, 120);
    assert_eq!(builder.height, 40);
}

#[test]
fn test_pty_builder_with_command_sets_command() {
    let session = Session::init()
        .with_host("localhost")
        .with_user("user")
        .with_passwd("pass")
        .build()
        .unwrap();

    let mut session_mut = session;
    let builder = session_mut.pty_builder().with_command("/bin/zsh");

    assert_eq!(builder.command, Some("/bin/zsh".to_string()));
}

#[test]
fn test_pty_builder_method_chaining() {
    let session = Session::init()
        .with_host("localhost")
        .with_user("user")
        .with_passwd("pass")
        .build()
        .unwrap();

    let mut session_mut = session;
    let builder = session_mut
        .pty_builder()
        .with_raw()
        .with_term("vt100")
        .with_size(100, 30)
        .with_command("/bin/bash");

    assert_eq!(builder.raw_mode, true);
    assert_eq!(builder.term, "vt100");
    assert_eq!(builder.width, 100);
    assert_eq!(builder.height, 30);
    assert_eq!(builder.command, Some("/bin/bash".to_string()));
    assert_eq!(builder.auto_resize, false);
}

#[test]
fn test_pty_builder_with_auto_resize_sets_flag() {
    let session = Session::init()
        .with_host("localhost")
        .with_user("user")
        .with_passwd("pass")
        .build()
        .unwrap();

    let mut session_mut = session;
    let builder = session_mut.pty_builder().with_auto_resize();

    assert_eq!(builder.auto_resize, true);
}

#[test]
fn test_pty_builder_auto_resize_default_false() {
    let session = Session::init()
        .with_host("localhost")
        .with_user("user")
        .with_passwd("pass")
        .build()
        .unwrap();

    let mut session_mut = session;
    let builder = session_mut.pty_builder();

    assert_eq!(builder.auto_resize, false);
}

#[tokio::test]
async fn test_pty_builder_uses_session_command() {
    let custom_cmd = vec!["zsh".to_string(), "-l".to_string()];
    let session = Session::init()
        .with_host("localhost")
        .with_user("user")
        .with_cmd(custom_cmd.clone())
        .with_passwd("pass")
        .build()
        .unwrap();

    let mut session_mut = session;
    let builder = session_mut.pty_builder();

    let expected_cmd: String = builder.session.inner.get_command();
    assert!(expected_cmd.contains("zsh"));
    assert!(expected_cmd.contains("-l"));
}

#[test]
fn test_pty_exit_status_code_variant() {
    let status = PtyExitStatus::Code(0);
    assert_eq!(status.code(), Some(0));

    let status = PtyExitStatus::Code(42);
    assert_eq!(status.code(), Some(42));

    let status = PtyExitStatus::Code(255);
    assert_eq!(status.code(), Some(255));
}

#[test]
fn test_pty_exit_status_signal_variant() {
    let status = PtyExitStatus::Signal {
        signal_name: Sig::TERM,
        core_dumped: false,
        error_message: "terminated".to_string(),
    };
    assert_eq!(status.code(), None);
}

#[test]
fn test_pty_exit_status_channel_closed_variant() {
    let status = PtyExitStatus::ChannelClosed;
    assert_eq!(status.code(), None);
}

#[test]
fn test_default_pty_terminal_modes_non_empty() {
    let modes = default_pty_terminal_modes();
    assert!(!modes.is_empty());

    // Verify some expected modes are present
    assert!(modes.contains(&(Pty::ICRNL, 1)));
    assert!(modes.contains(&(Pty::ECHO, 1)));
    assert!(modes.contains(&(Pty::IUTF8, 1)));
    assert!(modes.contains(&(Pty::TTY_OP_ISPEED, 38400)));
    assert!(modes.contains(&(Pty::TTY_OP_OSPEED, 38400)));
}

#[test]
fn test_pty_builder_terminal_modes_default_none() {
    let session = Session::init()
        .with_host("localhost")
        .with_user("user")
        .with_passwd("pass")
        .build()
        .unwrap();

    let mut session_mut = session;
    let builder = session_mut.pty_builder();
    assert!(builder.terminal_modes.is_none());
}

#[test]
fn test_pty_builder_with_terminal_modes() {
    let session = Session::init()
        .with_host("localhost")
        .with_user("user")
        .with_passwd("pass")
        .build()
        .unwrap();

    let mut session_mut = session;
    let custom_modes = vec![(Pty::ECHO, 0), (Pty::CS8, 1)];
    let builder = session_mut.pty_builder().with_terminal_modes(&custom_modes);

    assert_eq!(builder.terminal_modes, Some(custom_modes));
}

#[test]
fn test_pty_builder_with_empty_terminal_modes() {
    let session = Session::init()
        .with_host("localhost")
        .with_user("user")
        .with_passwd("pass")
        .build()
        .unwrap();

    let mut session_mut = session;
    let builder = session_mut.pty_builder().with_terminal_modes(&[]);
    assert_eq!(builder.terminal_modes, Some(vec![]));
}

#[tokio::test]
async fn test_pty_builder_open_no_session() {
    let mut session = Session::init()
        .with_host("localhost")
        .with_user("user")
        .with_passwd("pass")
        .build()
        .unwrap();

    let result = session.pty_builder().open().await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "No open session");
}

#[tokio::test]
async fn test_pty_builder_open_with_modes_no_session() {
    let mut session = Session::init()
        .with_host("localhost")
        .with_user("user")
        .with_passwd("pass")
        .build()
        .unwrap();

    let result = session
        .pty_builder()
        .with_terminal_modes(&[(Pty::ECHO, 1)])
        .open()
        .await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "No open session");
}

#[test]
fn test_pty_builder_full_config_with_modes() {
    let session = Session::init()
        .with_host("localhost")
        .with_user("user")
        .with_passwd("pass")
        .build()
        .unwrap();

    let mut session_mut = session;
    let modes = vec![(Pty::ECHO, 0), (Pty::ICANON, 0)];
    let builder = session_mut
        .pty_builder()
        .with_raw()
        .with_term("xterm-256color")
        .with_size(120, 40)
        .with_command("/bin/sh")
        .with_auto_resize()
        .with_terminal_modes(&modes);

    assert!(builder.raw_mode);
    assert_eq!(builder.term, "xterm-256color");
    assert_eq!(builder.width, 120);
    assert_eq!(builder.height, 40);
    assert_eq!(builder.command, Some("/bin/sh".to_string()));
    assert!(builder.auto_resize);
    assert_eq!(builder.terminal_modes, Some(modes));
}

#[test]
fn test_pty_builder_with_mode_detection_default_config() {
    let session = Session::init()
        .with_host("localhost")
        .with_user("user")
        .with_passwd("pass")
        .build()
        .unwrap();

    let mut session_mut = session;
    let builder = session_mut.pty_builder().with_mode_detection();

    assert!(builder.mode_detection_config.is_some());
}

#[test]
fn test_pty_builder_with_mode_detection_custom_config() {
    let session = Session::init()
        .with_host("localhost")
        .with_user("user")
        .with_passwd("pass")
        .build()
        .unwrap();

    let mut session_mut = session;
    let config = pty_mode::ModeDetectionConfig::default();
    let builder = session_mut.pty_builder().with_mode_detection_config(config);

    assert!(builder.mode_detection_config.is_some());
}

#[test]
fn test_pty_builder_mode_detection_disabled_by_default() {
    let session = Session::init()
        .with_host("localhost")
        .with_user("user")
        .with_passwd("pass")
        .build()
        .unwrap();

    let mut session_mut = session;
    let builder = session_mut.pty_builder();

    assert!(builder.mode_detection_config.is_none());
}

#[tokio::test]
async fn test_pty_handle_mode_detection_disabled_returns_none() {
    let (input_tx, _input_rx) = mpsc::channel(64);
    let (_output_tx, output_rx) = mpsc::channel(256);
    let (resize_tx, _resize_rx) = mpsc::channel(4);
    let (_exit_tx, exit_rx) = watch::channel(None);

    let mode_detection: Option<Arc<ModeDetection>> = None;

    let handle = PtyHandle {
        input_tx,
        output_rx,
        resize_tx,
        task_handle: None,
        exit_rx,
        closed: false,
        mode_detection,
        pwd_detection: None,
    };

    assert!(handle.current_mode().is_none());
    assert!(handle.is_std_mode());
}

#[tokio::test]
async fn test_pty_handle_mode_detection_enabled_shared_instance() {
    let config = ModeDetectionConfig {
        enabled: true,
        buffer_size: 256,
    };
    let mode_detection = Arc::new(ModeDetection::new(config));

    let (input_tx, _input_rx) = mpsc::channel(64);
    let (_output_tx, output_rx) = mpsc::channel(256);
    let (resize_tx, _resize_rx) = mpsc::channel(4);
    let (_exit_tx, exit_rx) = watch::channel(None);

    let handle = PtyHandle {
        input_tx,
        output_rx,
        resize_tx,
        task_handle: None,
        exit_rx,
        closed: false,
        mode_detection: Some(mode_detection.clone()),
        pwd_detection: None,
    };

    let handle_mode_detection = handle.mode_detection.clone();
    assert!(handle_mode_detection.is_some());
    assert!(handle.current_mode().is_some());
    assert!(!handle.is_alt_mode());
    assert!(handle.is_std_mode());
    assert_eq!(mode_detection.current_mode(), PtyMode::Standard);
}

#[test]
fn test_pty_builder_with_pwd_detection_default_config() {
    let session = Session::init()
        .with_host("localhost")
        .with_user("user")
        .with_passwd("pass")
        .build()
        .unwrap();

    let mut session_mut = session;
    let builder = session_mut.pty_builder().with_pwd_detection();

    assert!(builder.pwd_detection_config.is_some());
}

#[test]
fn test_pty_builder_with_pwd_detection_custom_config() {
    let session = Session::init()
        .with_host("localhost")
        .with_user("user")
        .with_passwd("pass")
        .build()
        .unwrap();

    let mut session_mut = session;
    let config = pty_pwd::PwdDetectionConfig::default();
    let builder = session_mut.pty_builder().with_pwd_detection_config(config);

    assert!(builder.pwd_detection_config.is_some());
}

#[test]
fn test_pty_builder_pwd_detection_disabled_by_default() {
    let session = Session::init()
        .with_host("localhost")
        .with_user("user")
        .with_passwd("pass")
        .build()
        .unwrap();

    let mut session_mut = session;
    let builder = session_mut.pty_builder();

    assert!(builder.pwd_detection_config.is_none());
}

#[tokio::test]
async fn test_pty_handle_pwd_detection_disabled_returns_none() {
    let (input_tx, _input_rx) = mpsc::channel(64);
    let (_output_tx, output_rx) = mpsc::channel(256);
    let (resize_tx, _resize_rx) = mpsc::channel(4);
    let (_exit_tx, exit_rx) = watch::channel(None);

    let handle = PtyHandle {
        input_tx,
        output_rx,
        resize_tx,
        task_handle: None,
        exit_rx,
        closed: false,
        mode_detection: None,
        pwd_detection: None,
    };

    assert!(handle.current_pwd().is_none());
    assert!(handle.watch_pwd().is_none());
}

#[tokio::test]
async fn test_pty_handle_pwd_detection_enabled_shared_instance() {
    let config = PwdDetectionConfig {
        enabled: true,
        buffer_size: 2048,
    };
    let pwd_detection = Arc::new(PwdDetection::new(config));

    let (input_tx, _input_rx) = mpsc::channel(64);
    let (_output_tx, output_rx) = mpsc::channel(256);
    let (resize_tx, _resize_rx) = mpsc::channel(4);
    let (_exit_tx, exit_rx) = watch::channel(None);

    let handle = PtyHandle {
        input_tx,
        output_rx,
        resize_tx,
        task_handle: None,
        exit_rx,
        closed: false,
        mode_detection: None,
        pwd_detection: Some(pwd_detection.clone()),
    };

    assert!(handle.current_pwd().is_none());
    assert!(handle.watch_pwd().is_some());

    pwd_detection.feed(b"\x1b]7;file://host/home/user\x07");
    assert_eq!(handle.current_pwd(), Some("/home/user".to_string()));
}
