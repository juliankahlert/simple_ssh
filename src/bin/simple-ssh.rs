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
use shell_escape::escape;
use std::borrow::Cow;
use std::path::PathBuf;
use tokio::time::{timeout, Duration};

use simple_ssh::{PtyExitStatus, PwdWatcher, Session};
use std::io::Write as _;

use crossterm::{
    cursor,
    event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute, queue,
    style::{
        Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
    },
    terminal::{
        disable_raw_mode, enable_raw_mode, size, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use futures::StreamExt;
use tokio::sync::mpsc;

/// Command line arguments for the simple-ssh binary.
#[derive(Debug, Parser, Clone, PartialEq)]
#[command(name = "simple-ssh")]
#[command(author = "Julian Kahlert")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "A simple SSH client with PTY support", long_about = None)]
struct Args {
    /// SSH host to connect to.
    #[arg(short = 'H', long)]
    #[arg(help = "SSH host to connect to")]
    host: String,

    /// SSH username.
    #[arg(short, long, default_value = "root")]
    #[arg(help = "SSH username")]
    user: String,

    /// SSH password.
    #[arg(short = 'P', long)]
    #[arg(help = "SSH password")]
    passwd: Option<String>,

    /// Path to private key file.
    #[arg(short = 'i', long)]
    #[arg(help = "Path to private key file")]
    key: Option<PathBuf>,

    /// SSH port.
    #[arg(short, long, default_value = "22")]
    #[arg(help = "SSH port")]
    port: u16,

    /// IPv6 scope ID (e.g., interface name or number).
    #[arg(long)]
    #[arg(help = "IPv6 scope ID (e.g., interface name or number)")]
    scope: Option<String>,

    /// Authentication method.
    #[arg(short, long, value_enum)]
    #[arg(help = "Authentication method")]
    auth: Option<AuthMethod>,

    /// Command to execute (if not provided, opens interactive shell).
    #[arg(trailing_var_arg = true)]
    #[arg(allow_hyphen_values = true)]
    #[arg(help = "Command to execute (if not provided, opens interactive shell)")]
    command: Vec<String>,

    /// Terminal multiplexer layout mode.
    #[arg(long, value_enum)]
    #[arg(help = "Multiplexer layout: 1x2, 2x1, or 2x2")]
    mux: Option<MuxMode>,
}

/// Authentication methods for SSH connections.
#[derive(Debug, Clone, ValueEnum, PartialEq)]
enum AuthMethod {
    /// Password-based authentication.
    #[value(name = "password")]
    Password,
    /// Public key authentication.
    #[value(name = "key")]
    Key,
    /// No authentication (none).
    #[value(name = "none")]
    None,
}

/// Terminal multiplexer layout mode.
#[derive(Debug, Clone, ValueEnum, PartialEq)]
enum MuxMode {
    /// 2 panes stacked vertically (1 column, 2 rows).
    #[value(name = "1x2")]
    OneByTwo,
    /// 2 panes side by side (2 columns, 1 row).
    #[value(name = "2x1")]
    TwoByOne,
    /// 4 panes in a 2x2 grid.
    #[value(name = "2x2")]
    TwoByTwo,
}

/// Builds a Session from command line arguments.
///
/// # Arguments
///
/// * `args` - Parsed command line arguments
///
/// # Returns
///
/// A configured Session or an error if required arguments are missing.
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

/// Joins command arguments into a single shell-escaped string.
///
/// Each argument is individually shell-escaped to preserve the original
/// quoting and spacing semantics when executed by a remote shell.
///
/// # Arguments
///
/// * `args` - Command line arguments
fn command_from_args(args: &Args) -> String {
    args.command
        .iter()
        .map(|s| escape(Cow::Borrowed(s.as_str())).to_string())
        .collect::<Vec<String>>()
        .join(" ")
}

/// Checks if a command was provided.
///
/// # Arguments
///
/// * `args` - Command line arguments
fn has_command(args: &Args) -> bool {
    !args.command.is_empty()
}

/// Position and size of a pane's box including its border.
///
/// The x, y, width, and height values describe the pane's outer box,
/// not its inner content area. The content area starts at y + 1 and
/// has dimensions (width) x (height). See draw_pane_box and render_pane
/// which treat layout.y as the border row with content at layout.y + 1.
#[derive(Debug, Clone)]
struct PaneLayout {
    x: u16,
    y: u16,
    width: u16,
    height: u16,
}

/// A single multiplexer pane with its virtual terminal and state.
struct Pane {
    layout: PaneLayout,
    parser: vt100::Parser,
    exited: bool,
    exit_status: Option<PtyExitStatus>,
}

/// Channel senders for communicating with a pane's background actor task.
struct PaneProxy {
    write_tx: mpsc::Sender<Vec<u8>>,
    resize_tx: mpsc::Sender<(u32, u32)>,
}

/// Events from pane actor tasks to the main mux loop.
enum MuxEvent {
    PtyOutput { pane: usize, data: Vec<u8> },
    PtyExit { pane: usize, status: PtyExitStatus },
}

/// Input state machine for Ctrl+B prefix handling.
enum InputState {
    Normal,
    Prefix,
}

fn calculate_layouts(cols: u16, rows: u16, mode: &MuxMode) -> Vec<PaneLayout> {
    match mode {
        MuxMode::OneByTwo => {
            let content_w = cols.saturating_sub(2);
            let total_content_h = rows.saturating_sub(4);
            let top_h = total_content_h / 2;
            let bot_h = total_content_h - top_h;
            vec![
                PaneLayout {
                    x: 1,
                    y: 0,
                    width: content_w,
                    height: top_h,
                },
                PaneLayout {
                    x: 1,
                    y: top_h + 2,
                    width: content_w,
                    height: bot_h,
                },
            ]
        }
        MuxMode::TwoByOne => {
            let content_h = rows.saturating_sub(2);
            let total_content_w = cols.saturating_sub(4);
            let left_w = total_content_w / 2;
            let right_w = total_content_w - left_w;
            vec![
                PaneLayout {
                    x: 1,
                    y: 0,
                    width: left_w,
                    height: content_h,
                },
                PaneLayout {
                    x: 1 + left_w + 2,
                    y: 0,
                    width: right_w,
                    height: content_h,
                },
            ]
        }
        MuxMode::TwoByTwo => {
            let total_content_w = cols.saturating_sub(4);
            let total_content_h = rows.saturating_sub(4);
            let left_w = total_content_w / 2;
            let right_w = total_content_w - left_w;
            let top_h = total_content_h / 2;
            let bot_h = total_content_h - top_h;
            vec![
                PaneLayout {
                    x: 1,
                    y: 0,
                    width: left_w,
                    height: top_h,
                },
                PaneLayout {
                    x: 1 + left_w + 2,
                    y: 0,
                    width: right_w,
                    height: top_h,
                },
                PaneLayout {
                    x: 1,
                    y: top_h + 2,
                    width: left_w,
                    height: bot_h,
                },
                PaneLayout {
                    x: 1 + left_w + 2,
                    y: top_h + 2,
                    width: right_w,
                    height: bot_h,
                },
            ]
        }
    }
}

/// Formats the pane title with optional PWD, truncating with ellipsis if needed.
///
/// `max_width` is the number of characters available between `┌` and `┐`.
fn format_pane_title(idx_label: &str, pwd: Option<&str>, max_width: usize) -> String {
    let Some(path) = pwd else {
        return idx_label.to_string();
    };

    // Need at least space for "[N] X" (label + space + 1 path char)
    let min_path_display = idx_label.len() + 2;
    if max_width < min_path_display {
        return idx_label.to_string();
    }

    let max_path_cols = max_width - idx_label.len() - 1; // -1 for space separator
    let path_chars: usize = path.chars().count();
    if path_chars <= max_path_cols {
        format!("{} {}", idx_label, path)
    } else if max_path_cols >= 2 {
        // Truncate from beginning with ellipsis
        let suffix_chars = max_path_cols - 1; // -1 for '…'
        let suffix: String = path.chars().skip(path_chars - suffix_chars).collect();
        format!("{} \u{2026}{}", idx_label, suffix)
    } else {
        idx_label.to_string()
    }
}

fn draw_pane_box(
    stdout: &mut std::io::Stdout,
    pane_idx: usize,
    layout: &PaneLayout,
    focused: bool,
    pwd: Option<&str>,
) -> Result<()> {
    let color = if focused {
        Color::Cyan
    } else {
        Color::DarkGrey
    };
    let idx_label = format!("[{}]", pane_idx + 1);
    let title = format_pane_title(&idx_label, pwd, layout.width as usize);
    let left_x = layout.x - 1;
    let right_x = layout.x + layout.width;
    let bottom_y = layout.y + layout.height + 1;

    queue!(stdout, SetForegroundColor(color))?;

    queue!(stdout, cursor::MoveTo(left_x, layout.y))?;
    queue!(stdout, Print('┌'))?;
    queue!(stdout, Print(&title))?;
    let title_cols = title.chars().count() as u16;
    for _ in (left_x + 1 + title_cols)..right_x {
        queue!(stdout, Print('─'))?;
    }
    queue!(stdout, cursor::MoveTo(right_x, layout.y))?;
    queue!(stdout, Print('┐'))?;

    for row in layout.y + 1..bottom_y {
        queue!(stdout, cursor::MoveTo(left_x, row))?;
        queue!(stdout, Print('│'))?;
        queue!(stdout, cursor::MoveTo(right_x, row))?;
        queue!(stdout, Print('│'))?;
    }

    queue!(stdout, cursor::MoveTo(left_x, bottom_y))?;
    queue!(stdout, Print('└'))?;
    for _ in left_x + 1..right_x {
        queue!(stdout, Print('─'))?;
    }
    queue!(stdout, cursor::MoveTo(right_x, bottom_y))?;
    queue!(stdout, Print('┘'))?;

    Ok(())
}

fn draw_borders(
    stdout: &mut std::io::Stdout,
    focused: usize,
    layouts: &[PaneLayout],
    pwd_watchers: &[PwdWatcher],
) -> Result<()> {
    for (i, layout) in layouts.iter().enumerate() {
        let pwd = pwd_watchers[i].current();
        draw_pane_box(stdout, i, layout, i == focused, pwd.as_deref())?;
    }
    Ok(())
}

fn vt100_to_crossterm_color(color: vt100::Color) -> Color {
    match color {
        vt100::Color::Default => Color::Reset,
        vt100::Color::Idx(i) => Color::AnsiValue(i),
        vt100::Color::Rgb(r, g, b) => Color::Rgb { r, g, b },
    }
}

fn render_pane(stdout: &mut std::io::Stdout, pane: &Pane) -> Result<()> {
    let screen = pane.parser.screen();
    let content_start_y = pane.layout.y + 1;

    for row in 0..pane.layout.height {
        queue!(stdout, cursor::MoveTo(pane.layout.x, content_start_y + row))?;

        let mut col = 0u16;
        let mut prev_fg: Option<Color> = None;
        let mut prev_bg: Option<Color> = None;
        let mut prev_attrs: Vec<Attribute> = Vec::new();

        while col < pane.layout.width {
            let cell = screen.cell(row, col);
            if let Some(cell) = cell {
                if cell.is_wide_continuation() {
                    col += 1;
                    continue;
                }

                let fg = vt100_to_crossterm_color(cell.fgcolor());
                if Some(fg) != prev_fg {
                    queue!(stdout, SetForegroundColor(fg))?;
                    prev_fg = Some(fg);
                }

                let bg = vt100_to_crossterm_color(cell.bgcolor());
                if Some(bg) != prev_bg {
                    queue!(stdout, SetBackgroundColor(bg))?;
                    prev_bg = Some(bg);
                }

                let mut attrs: Vec<Attribute> = Vec::new();
                let mut removed: Vec<Attribute> = Vec::new();
                if cell.bold() {
                    attrs.push(Attribute::Bold);
                }
                if cell.dim() {
                    attrs.push(Attribute::Dim);
                }
                if !cell.bold()
                    && !cell.dim()
                    && (prev_attrs.contains(&Attribute::Bold)
                        || prev_attrs.contains(&Attribute::Dim))
                    && !removed.contains(&Attribute::NormalIntensity)
                {
                    removed.push(Attribute::NormalIntensity);
                }
                if cell.italic() {
                    attrs.push(Attribute::Italic);
                } else if prev_attrs.contains(&Attribute::Italic) {
                    removed.push(Attribute::NoItalic);
                }
                if cell.underline() {
                    attrs.push(Attribute::Underlined);
                } else if prev_attrs.contains(&Attribute::Underlined) {
                    removed.push(Attribute::NoUnderline);
                }
                if cell.inverse() {
                    attrs.push(Attribute::Reverse);
                } else if prev_attrs.contains(&Attribute::Reverse) {
                    removed.push(Attribute::NoReverse);
                }

                for attr in &attrs {
                    if !prev_attrs.contains(attr) {
                        queue!(stdout, SetAttribute(*attr))?;
                    }
                }
                for attr in &removed {
                    queue!(stdout, SetAttribute(*attr))?;
                }
                prev_attrs = attrs;

                let contents = cell.contents();
                if contents.is_empty() {
                    queue!(stdout, Print(' '))?;
                } else {
                    queue!(stdout, Print(contents))?;
                    if cell.is_wide() {
                        col += 1;
                    }
                }
            } else {
                queue!(
                    stdout,
                    ResetColor,
                    SetAttribute(Attribute::Reset),
                    Print(' ')
                )?;
                prev_fg = None;
                prev_bg = None;
                prev_attrs.clear();
            }
            col += 1;
        }
        queue!(stdout, ResetColor, SetAttribute(Attribute::Reset))?;
    }

    if pane.exited {
        let msg = match &pane.exit_status {
            Some(PtyExitStatus::Code(c)) => format!("[exited: {}]", c),
            Some(PtyExitStatus::Signal { signal_name, .. }) => {
                format!("[signal: {:?}]", signal_name)
            }
            Some(PtyExitStatus::ChannelClosed) => "[closed]".to_string(),
            None => "[exited]".to_string(),
        };
        let msg_col = pane.layout.x + pane.layout.width.saturating_sub(msg.len() as u16) / 2;
        let msg_row = content_start_y + pane.layout.height / 2;
        queue!(
            stdout,
            cursor::MoveTo(msg_col, msg_row),
            SetAttribute(Attribute::Bold),
            SetForegroundColor(Color::Yellow),
            SetBackgroundColor(Color::DarkRed),
            Print(&msg),
            ResetColor,
            SetAttribute(Attribute::Reset),
        )?;
    }

    Ok(())
}

fn position_cursor(stdout: &mut std::io::Stdout, pane: &Pane) -> Result<()> {
    if pane.exited {
        queue!(stdout, cursor::Hide)?;
    } else {
        let (vrow, vcol) = pane.parser.screen().cursor_position();
        let content_start_y = pane.layout.y + 1;
        queue!(
            stdout,
            cursor::MoveTo(pane.layout.x + vcol, content_start_y + vrow),
            cursor::Show,
        )?;
    }
    Ok(())
}

fn key_event_to_bytes(event: &KeyEvent) -> Option<Vec<u8>> {
    if event.kind != KeyEventKind::Press {
        return None;
    }

    match event.code {
        KeyCode::Char(c) => {
            if event.modifiers.contains(KeyModifiers::CONTROL) {
                if c.is_ascii_lowercase() || c.is_ascii_uppercase() {
                    let ctrl_byte = (c.to_ascii_lowercase() as u8) - b'a' + 1;
                    Some(vec![ctrl_byte])
                } else {
                    None
                }
            } else if event.modifiers.contains(KeyModifiers::ALT) {
                let mut buf = vec![0x1b];
                let mut char_buf = [0u8; 4];
                let s = c.encode_utf8(&mut char_buf);
                buf.extend_from_slice(s.as_bytes());
                Some(buf)
            } else {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                Some(s.as_bytes().to_vec())
            }
        }
        KeyCode::Enter => Some(vec![b'\r']),
        KeyCode::Backspace => Some(vec![0x7f]),
        KeyCode::Tab => Some(vec![b'\t']),
        KeyCode::BackTab => Some(b"\x1b[Z".to_vec()),
        KeyCode::Esc => Some(vec![0x1b]),
        KeyCode::Up => Some(b"\x1b[A".to_vec()),
        KeyCode::Down => Some(b"\x1b[B".to_vec()),
        KeyCode::Right => Some(b"\x1b[C".to_vec()),
        KeyCode::Left => Some(b"\x1b[D".to_vec()),
        KeyCode::Home => Some(b"\x1b[H".to_vec()),
        KeyCode::End => Some(b"\x1b[F".to_vec()),
        KeyCode::PageUp => Some(b"\x1b[5~".to_vec()),
        KeyCode::PageDown => Some(b"\x1b[6~".to_vec()),
        KeyCode::Delete => Some(b"\x1b[3~".to_vec()),
        KeyCode::Insert => Some(b"\x1b[2~".to_vec()),
        KeyCode::F(n) => {
            let seq = match n {
                1 => "\x1bOP",
                2 => "\x1bOQ",
                3 => "\x1bOR",
                4 => "\x1bOS",
                5 => "\x1b[15~",
                6 => "\x1b[17~",
                7 => "\x1b[18~",
                8 => "\x1b[19~",
                9 => "\x1b[20~",
                10 => "\x1b[21~",
                11 => "\x1b[23~",
                12 => "\x1b[24~",
                _ => return None,
            };
            Some(seq.as_bytes().to_vec())
        }
        _ => None,
    }
}

fn navigate_pane(focused: usize, direction: KeyCode, mode: &MuxMode) -> Option<usize> {
    match mode {
        MuxMode::OneByTwo => match (focused, direction) {
            (0, KeyCode::Down) => Some(1),
            (1, KeyCode::Up) => Some(0),
            _ => None,
        },
        MuxMode::TwoByOne => match (focused, direction) {
            (0, KeyCode::Right) => Some(1),
            (1, KeyCode::Left) => Some(0),
            _ => None,
        },
        MuxMode::TwoByTwo => match (focused, direction) {
            (0, KeyCode::Right) => Some(1),
            (0, KeyCode::Down) => Some(2),
            (1, KeyCode::Left) => Some(0),
            (1, KeyCode::Down) => Some(3),
            (2, KeyCode::Right) => Some(3),
            (2, KeyCode::Up) => Some(0),
            (3, KeyCode::Left) => Some(2),
            (3, KeyCode::Up) => Some(1),
            _ => None,
        },
    }
}

async fn pane_actor(
    mut handle: simple_ssh::PtyHandle,
    pane_idx: usize,
    mux_tx: mpsc::Sender<MuxEvent>,
    mut write_rx: mpsc::Receiver<Vec<u8>>,
    mut resize_rx: mpsc::Receiver<(u32, u32)>,
) {
    loop {
        tokio::select! {
            data = handle.read() => {
                match data {
                    Some(data) => {
                        if mux_tx.send(MuxEvent::PtyOutput {
                            pane: pane_idx,
                            data,
                        }).await.is_err() {
                            return;
                        }
                    }
                    None => {
                        let status = handle.try_wait()
                            .unwrap_or(PtyExitStatus::ChannelClosed);
                        let _ = mux_tx.send(MuxEvent::PtyExit {
                            pane: pane_idx,
                            status,
                        }).await;
                        return;
                    }
                }
            }
            Some(data) = write_rx.recv() => {
                if let Err(e) = handle.write(&data).await {
                    eprintln!("Pane {} write error: {}", pane_idx, e);
                }
            }
            Some((cols, rows)) = resize_rx.recv() => {
                if let Err(e) = handle.resize(cols, rows).await {
                    eprintln!("Pane {} resize error: {}x{} -> {}", pane_idx, cols, rows, e);
                }
            }
        }
    }
}

async fn mux_session(args: &Args, mode: &MuxMode) -> Result<()> {
    let mut stdout = std::io::stdout();

    execute!(
        stdout,
        EnterAlternateScreen,
        Clear(ClearType::All),
        cursor::Hide,
    )?;
    enable_raw_mode()?;

    let (cols, rows) = size()?;
    let mut layouts = calculate_layouts(cols, rows, mode);

    let (mux_tx, mut mux_rx) = mpsc::channel::<MuxEvent>(256);

    let mut pane_proxies = Vec::new();
    let mut sessions = Vec::new();
    let mut panes = Vec::new();
    let mut pwd_watchers = Vec::new();

    for (i, layout) in layouts.iter().enumerate() {
        let session = build_session_from_args(args)?;
        let mut ssh = match timeout(Duration::from_secs(30), session.connect()).await {
            Ok(Ok(s)) => s,
            Ok(Err(e)) => {
                cleanup_mux(&mut stdout, &mut sessions).await;
                return Err(anyhow!("Pane {} connection failed: {}", i, e));
            }
            Err(_) => {
                cleanup_mux(&mut stdout, &mut sessions).await;
                return Err(anyhow!("Pane {} connection timed out", i));
            }
        };

        let handle = match ssh
            .pty_builder()
            .with_term("xterm-256color")
            .with_size(layout.width as u32, layout.height as u32)
            .with_pwd_detection(true)
            .open()
            .await
        {
            Ok(h) => h,
            Err(e) => {
                cleanup_mux(&mut stdout, &mut sessions).await;
                return Err(anyhow!("Pane {} PTY open failed: {}", i, e));
            }
        };

        let watcher = match handle.watch_pwd() {
            Ok(w) => w,
            Err(e) => {
                cleanup_mux(&mut stdout, &mut sessions).await;
                return Err(anyhow!("Pane {} PWD watcher failed: {}", i, e));
            }
        };

        let (write_tx, write_rx) = mpsc::channel(64);
        let (resize_tx, resize_rx) = mpsc::channel(4);

        let tx = mux_tx.clone();
        tokio::spawn(pane_actor(handle, i, tx, write_rx, resize_rx));

        pane_proxies.push(PaneProxy {
            write_tx,
            resize_tx,
        });
        sessions.push(ssh);
        pwd_watchers.push(watcher);
        panes.push(Pane {
            layout: layout.clone(),
            parser: vt100::Parser::new(layout.height, layout.width, 0),
            exited: false,
            exit_status: None,
        });
    }
    drop(mux_tx);

    let mut focused: usize = 0;
    let mut input_state = InputState::Normal;

    draw_borders(&mut stdout, focused, &layouts, &pwd_watchers)?;
    stdout.flush()?;

    let mut event_stream = EventStream::new();

    loop {
        tokio::select! {
            event = event_stream.next() => {
                let Some(Ok(event)) = event else { break };
                match event {
                    Event::Key(key_event) => {
                        if key_event.kind != KeyEventKind::Press {
                            continue;
                        }
                        match input_state {
                            InputState::Normal => {
                                if key_event.code == KeyCode::Char('b')
                                    && key_event.modifiers.contains(KeyModifiers::CONTROL)
                                {
                                    input_state = InputState::Prefix;
                                } else if let Some(bytes) = key_event_to_bytes(&key_event) {
                                    if !panes[focused].exited {
                                        let _ = pane_proxies[focused]
                                            .write_tx
                                            .send(bytes)
                                            .await;
                                    }
                                }
                            }
                            InputState::Prefix => {
                                input_state = InputState::Normal;
                                match key_event.code {
                                    KeyCode::Char('b')
                                        if key_event.modifiers
                                            .contains(KeyModifiers::CONTROL) =>
                                    {
                                        if !panes[focused].exited {
                                            let _ = pane_proxies[focused]
                                                .write_tx
                                                .send(vec![0x02])
                                                .await;
                                        }
                                    }
                                    KeyCode::Up | KeyCode::Down
                                    | KeyCode::Left | KeyCode::Right => {
                                        if let Some(new_focus) =
                                            navigate_pane(focused, key_event.code, mode)
                                        {
                                            focused = new_focus;
                                            draw_borders(
                                                &mut stdout, focused, &layouts,
                                                &pwd_watchers,
                                            )?;
                                            position_cursor(
                                                &mut stdout, &panes[focused],
                                            )?;
                                            stdout.flush()?;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    #[allow(unused)]
                    Event::Resize(new_cols, new_rows) => {
                        layouts = calculate_layouts(new_cols, new_rows, mode);

                        for (i, pane) in panes.iter_mut().enumerate() {
                            pane.layout = layouts[i].clone();
                            pane.parser.screen_mut().set_size(
                                pane.layout.height,
                                pane.layout.width,
                            );
                            let _ = pane_proxies[i].resize_tx.send((
                                pane.layout.width as u32,
                                pane.layout.height as u32,
                            )).await;
                        }

                        queue!(stdout, Clear(ClearType::All))?;
                        draw_borders(
                            &mut stdout, focused, &layouts,
                            &pwd_watchers,
                        )?;
                        for pane in &panes {
                            render_pane(&mut stdout, pane)?;
                        }
                        position_cursor(&mut stdout, &panes[focused])?;
                        stdout.flush()?;
                    }
                    _ => {}
                }
            }

            Some(mux_event) = mux_rx.recv() => {
                match mux_event {
                    MuxEvent::PtyOutput { pane, data } => {
                        panes[pane].parser.process(&data);
                        let pwd = pwd_watchers[pane].current();
                        draw_pane_box(
                            &mut stdout, pane, &panes[pane].layout,
                            pane == focused, pwd.as_deref(),
                        )?;
                        render_pane(&mut stdout, &panes[pane])?;
                        if pane == focused {
                            position_cursor(&mut stdout, &panes[focused])?;
                        }
                        stdout.flush()?;
                    }
                    MuxEvent::PtyExit { pane, status } => {
                        panes[pane].exited = true;
                        panes[pane].exit_status = Some(status);
                        render_pane(&mut stdout, &panes[pane])?;
                        stdout.flush()?;

                        if panes.iter().all(|p| p.exited) {
                            break;
                        }
                    }
                }
            }

            else => break,
        }
    }

    cleanup_mux(&mut stdout, &mut sessions).await;
    Ok(())
}

async fn cleanup_mux(stdout: &mut std::io::Stdout, sessions: &mut Vec<Session>) {
    let _ = disable_raw_mode();
    let _ = execute!(stdout, cursor::Show, LeaveAlternateScreen, ResetColor);
    for session in sessions {
        let _ = session.close().await;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();

    if let Some(ref mode) = args.mux {
        if has_command(&args) {
            return Err(anyhow!("--mux cannot be used with a command"));
        }
        return mux_session(&args, mode).await;
    }

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

/// Runs an interactive shell session with PTY support.
///
/// # Arguments
///
/// * `ssh` - Connected SSH session
async fn interactive_shell(ssh: &mut Session) -> Result<u32> {
    let exit_code = ssh
        .pty_builder()
        .with_raw()
        .with_auto_resize()
        .run()
        .await?;
    println!("\r\nConnection closed with exit code: {}", exit_code);
    Ok(exit_code)
}

/// Executes a non-interactive command.
///
/// # Arguments
///
/// * `ssh` - Connected SSH session
/// * `command` - Command to execute
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
    fn test_command_from_args_with_special_chars() {
        let args = Args::parse_from(&["simple-ssh", "-H", "localhost", "echo", "hello world"]);
        assert_eq!(command_from_args(&args), r#"echo 'hello world'"#);
    }

    #[test]
    fn test_command_from_args_with_quotes() {
        let args = Args::parse_from(&["simple-ssh", "-H", "localhost", "echo", "it's a test"]);
        assert_eq!(command_from_args(&args), r#"echo 'it'\''s a test'"#);
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

    #[test]
    fn test_mux_mode_parsing() {
        let args = Args::parse_from(&["simple-ssh", "-H", "localhost", "--mux", "1x2"]);
        assert_eq!(args.mux, Some(MuxMode::OneByTwo));

        let args = Args::parse_from(&["simple-ssh", "-H", "localhost", "--mux", "2x1"]);
        assert_eq!(args.mux, Some(MuxMode::TwoByOne));

        let args = Args::parse_from(&["simple-ssh", "-H", "localhost", "--mux", "2x2"]);
        assert_eq!(args.mux, Some(MuxMode::TwoByTwo));
    }

    #[test]
    fn test_mux_mode_default_none() {
        let args = Args::parse_from(&["simple-ssh", "-H", "localhost"]);
        assert_eq!(args.mux, None);
    }

    #[test]
    fn test_calculate_layouts_1x2() {
        let layouts = calculate_layouts(80, 24, &MuxMode::OneByTwo);
        assert_eq!(layouts.len(), 2);
        assert_eq!(layouts[0].width, layouts[1].width);
        assert!(layouts[0].y < layouts[1].y);
        assert!(layouts[0].x >= 1);
        assert_eq!(layouts[0].y, 0);
        assert_eq!(layouts[0].height + layouts[1].height + 4, 24);
    }

    #[test]
    fn test_calculate_layouts_2x1() {
        let layouts = calculate_layouts(80, 24, &MuxMode::TwoByOne);
        assert_eq!(layouts.len(), 2);
        assert_eq!(layouts[0].height, layouts[1].height);
        assert!(layouts[0].x < layouts[1].x);
        assert_eq!(layouts[0].width + layouts[1].width + 4, 80);
    }

    #[test]
    fn test_calculate_layouts_2x2() {
        let layouts = calculate_layouts(80, 24, &MuxMode::TwoByTwo);
        assert_eq!(layouts.len(), 4);
        assert_eq!(layouts[0].height, layouts[1].height);
        assert_eq!(layouts[2].height, layouts[3].height);
        assert_eq!(layouts[0].width, layouts[2].width);
        assert_eq!(layouts[1].width, layouts[3].width);
        for i in 0..4 {
            for j in (i + 1)..4 {
                let a = &layouts[i];
                let b = &layouts[j];
                let no_x_overlap = a.x + a.width <= b.x || b.x + b.width <= a.x;
                let no_y_overlap = a.y + a.height <= b.y || b.y + b.height <= a.y;
                assert!(
                    no_x_overlap || no_y_overlap,
                    "Panes {} and {} overlap",
                    i,
                    j
                );
            }
        }
    }

    #[test]
    fn test_navigate_pane_1x2() {
        assert_eq!(navigate_pane(0, KeyCode::Down, &MuxMode::OneByTwo), Some(1));
        assert_eq!(navigate_pane(1, KeyCode::Up, &MuxMode::OneByTwo), Some(0));
        assert_eq!(navigate_pane(0, KeyCode::Left, &MuxMode::OneByTwo), None);
        assert_eq!(navigate_pane(0, KeyCode::Up, &MuxMode::OneByTwo), None);
    }

    #[test]
    fn test_navigate_pane_2x1() {
        assert_eq!(
            navigate_pane(0, KeyCode::Right, &MuxMode::TwoByOne),
            Some(1)
        );
        assert_eq!(navigate_pane(1, KeyCode::Left, &MuxMode::TwoByOne), Some(0));
        assert_eq!(navigate_pane(0, KeyCode::Up, &MuxMode::TwoByOne), None);
    }

    #[test]
    fn test_navigate_pane_2x2() {
        assert_eq!(
            navigate_pane(0, KeyCode::Right, &MuxMode::TwoByTwo),
            Some(1)
        );
        assert_eq!(navigate_pane(0, KeyCode::Down, &MuxMode::TwoByTwo), Some(2));
        assert_eq!(navigate_pane(0, KeyCode::Left, &MuxMode::TwoByTwo), None);
        assert_eq!(navigate_pane(0, KeyCode::Up, &MuxMode::TwoByTwo), None);
        assert_eq!(navigate_pane(3, KeyCode::Left, &MuxMode::TwoByTwo), Some(2));
        assert_eq!(navigate_pane(3, KeyCode::Up, &MuxMode::TwoByTwo), Some(1));
    }

    #[test]
    fn test_key_event_to_bytes_char() {
        let event = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        assert_eq!(key_event_to_bytes(&event), Some(vec![b'a']));
    }

    #[test]
    fn test_key_event_to_bytes_ctrl() {
        let event = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(key_event_to_bytes(&event), Some(vec![0x03]));
    }

    #[test]
    fn test_key_event_to_bytes_enter() {
        let event = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(key_event_to_bytes(&event), Some(vec![b'\r']));
    }

    #[test]
    fn test_key_event_to_bytes_arrow() {
        let event = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(key_event_to_bytes(&event), Some(b"\x1b[A".to_vec()));
    }

    #[test]
    fn test_key_event_to_bytes_f1() {
        let event = KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE);
        assert_eq!(key_event_to_bytes(&event), Some(b"\x1bOP".to_vec()));
    }

    #[test]
    fn test_vt100_to_crossterm_color() {
        assert!(matches!(
            vt100_to_crossterm_color(vt100::Color::Default),
            Color::Reset,
        ));
        assert!(matches!(
            vt100_to_crossterm_color(vt100::Color::Idx(1)),
            Color::AnsiValue(1),
        ));
        assert!(matches!(
            vt100_to_crossterm_color(vt100::Color::Rgb(255, 0, 128)),
            Color::Rgb {
                r: 255,
                g: 0,
                b: 128
            },
        ));
    }
}
