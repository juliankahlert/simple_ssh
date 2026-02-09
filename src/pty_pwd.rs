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

use anyhow::Result;
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::watch;

/// A PWD change event with old and new paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PwdChangeEvent {
    /// The previous working directory, if known.
    pub previous: Option<String>,
    /// The new working directory.
    pub current: String,
    /// Timestamp of when the change was detected.
    pub timestamp: Instant,
}

/// Configuration for PWD detection behavior.
#[derive(Debug, Clone)]
pub struct PwdDetectionConfig {
    /// Whether to enable PWD detection (default: false)
    pub enabled: bool,
    /// Buffer size for OSC payload parsing (default: 2048 bytes)
    pub buffer_size: usize,
}

impl Default for PwdDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            buffer_size: 2048,
        }
    }
}

/// States for the OSC escape sequence parser.
enum OscParserState {
    Normal,
    Escape,
    Osc,
    OscEscape,
}

/// Stateful parser for OSC escape sequences that report working directory.
///
/// Recognizes:
/// - OSC 7: `\x1b]7;file://hostname/path\x07` (or ST-terminated)
/// - OSC 633: `\x1b]633;P;Cwd=/path\x07` (VS Code shell integration)
struct OscParser {
    buffer: Vec<u8>,
    state: OscParserState,
    max_buffer_size: usize,
}

impl OscParser {
    /// Creates a new parser with the specified maximum buffer size.
    fn new(buffer_size: usize) -> Self {
        let buffer_size = buffer_size.max(64);
        Self {
            buffer: Vec::with_capacity(buffer_size),
            state: OscParserState::Normal,
            max_buffer_size: buffer_size,
        }
    }

    /// Feeds raw bytes to the parser and returns any detected PWD paths.
    fn feed(&mut self, data: &[u8]) -> Vec<String> {
        let mut paths = Vec::new();

        for &byte in data {
            match self.state {
                OscParserState::Normal => {
                    if byte == 0x1b {
                        self.state = OscParserState::Escape;
                    }
                }
                OscParserState::Escape => {
                    if byte == b']' {
                        self.state = OscParserState::Osc;
                        self.buffer.clear();
                    } else {
                        self.state = OscParserState::Normal;
                    }
                }
                OscParserState::Osc => {
                    if byte == 0x07 {
                        // BEL terminates OSC
                        if let Some(path) = self.process_osc_payload() {
                            paths.push(path);
                        }
                        self.reset();
                    } else if byte == 0x1b {
                        // Possible ST (\x1b\\)
                        self.state = OscParserState::OscEscape;
                    } else if self.buffer.len() < self.max_buffer_size {
                        self.buffer.push(byte);
                    } else {
                        // Buffer overflow — discard
                        self.reset();
                    }
                }
                OscParserState::OscEscape => {
                    if byte == b'\\' {
                        // ST terminates OSC
                        if let Some(path) = self.process_osc_payload() {
                            paths.push(path);
                        }
                        self.reset();
                    } else {
                        // Not ST — discard and treat as new escape
                        self.reset();
                        if byte == b']' {
                            self.state = OscParserState::Osc;
                            self.buffer.clear();
                        } else if byte == 0x1b {
                            self.state = OscParserState::Escape;
                        }
                    }
                }
            }
        }

        paths
    }

    /// Processes the collected OSC payload and extracts a PWD path if valid.
    fn process_osc_payload(&self) -> Option<String> {
        let payload = std::str::from_utf8(&self.buffer).ok()?;

        // OSC 7: "7;file://hostname/path"
        if let Some(rest) = payload.strip_prefix("7;") {
            return self.parse_osc7_url(rest);
        }

        // OSC 633: "633;P;Cwd=/path"
        if let Some(rest) = payload.strip_prefix("633;P;Cwd=") {
            let decoded = percent_decode(rest);
            if !decoded.is_empty() {
                return Some(decoded);
            }
        }

        None
    }

    /// Parses an OSC 7 file:// URL and extracts the path component.
    ///
    /// Handles `file://hostname/path` and `file:///path` forms.
    fn parse_osc7_url(&self, url: &str) -> Option<String> {
        let rest = url.strip_prefix("file://")?;
        // Skip hostname (everything up to the first '/')
        let path_start = rest.find('/')?;
        let path = &rest[path_start..];
        let decoded = percent_decode(path);
        if !decoded.is_empty() {
            Some(decoded)
        } else {
            None
        }
    }

    /// Resets the parser state and clears the buffer.
    fn reset(&mut self) {
        self.state = OscParserState::Normal;
        self.buffer.clear();
    }
}

/// Decodes percent-encoded bytes in a string (e.g., `%20` → ` `).
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut result = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Some(decoded) = decode_hex_pair(bytes[i + 1], bytes[i + 2]) {
                result.push(decoded);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i]);
        i += 1;
    }

    String::from_utf8(result).unwrap_or_else(|_| input.to_string())
}

/// Decodes a pair of hex ASCII characters into a byte.
fn decode_hex_pair(hi: u8, lo: u8) -> Option<u8> {
    let hi = hex_digit(hi)?;
    let lo = hex_digit(lo)?;
    Some(hi << 4 | lo)
}

/// Converts an ASCII hex digit to its numeric value.
fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Internal PWD detection state for tracking working directory changes.
///
/// Manages the OSC parser and broadcasts PWD changes to watchers.
pub(crate) struct PwdDetection {
    current_pwd: Arc<Mutex<Option<(String, Instant)>>>,
    parser: Mutex<OscParser>,
    event_tx: watch::Sender<Option<(String, Instant)>>,
    event_rx: watch::Receiver<Option<(String, Instant)>>,
    enabled: bool,
}

impl PwdDetection {
    /// Creates a new PWD detector with the given configuration.
    pub(crate) fn new(config: PwdDetectionConfig) -> Self {
        let (tx, rx) = watch::channel(None);
        Self {
            current_pwd: Arc::new(Mutex::new(None)),
            parser: Mutex::new(OscParser::new(config.buffer_size)),
            event_tx: tx,
            event_rx: rx,
            enabled: config.enabled,
        }
    }

    /// Returns the current working directory, if known.
    pub(crate) fn current_pwd(&self) -> Option<String> {
        self.current_pwd.lock().as_ref().map(|(s, _)| s.clone())
    }

    /// Updates the current PWD and notifies all watchers.
    ///
    /// Only sends notifications when the path actually changes.
    pub(crate) fn update_pwd(&self, new_pwd: String) {
        let mut current = self.current_pwd.lock();
        let changed = match current.as_ref() {
            Some((old, _)) => old != &new_pwd,
            None => true,
        };
        if changed {
            let timestamp = Instant::now();
            *current = Some((new_pwd.clone(), timestamp));
            let _ = self.event_tx.send(Some((new_pwd, timestamp)));
        }
    }

    /// Feeds data to the OSC parser.
    ///
    /// If PWD detection is disabled, this is a no-op.
    pub(crate) fn feed(&self, data: &[u8]) {
        if !self.enabled {
            return;
        }
        let mut parser = self.parser.lock();
        let paths = parser.feed(data);
        drop(parser);
        for path in paths {
            self.update_pwd(path);
        }
    }

    /// Creates a new watcher for observing PWD changes.
    pub(crate) fn create_watcher(&self) -> Result<PwdWatcher> {
        Ok(PwdWatcher {
            inner: self.event_rx.clone(),
            last_known: self.current_pwd.lock().clone(),
        })
    }
}

/// An async-enabled watcher for PWD changes.
///
/// Tracks the remote shell's working directory as reported via
/// OSC 7 or OSC 633 escape sequences.
#[derive(Debug, Clone)]
pub struct PwdWatcher {
    inner: watch::Receiver<Option<(String, Instant)>>,
    last_known: Option<(String, Instant)>,
}

impl PwdWatcher {
    /// Returns the current working directory without waiting, if known.
    pub fn current(&self) -> Option<String> {
        self.inner.borrow().as_ref().map(|(s, _)| s.clone())
    }

    /// Waits for the PWD to change and returns the new path and detection timestamp.
    ///
    /// Returns `None` if the PTY session has ended.
    pub async fn changed(&mut self) -> Option<(String, Instant)> {
        loop {
            match self.inner.changed().await {
                Ok(()) => {
                    let pwd = self.inner.borrow().clone();
                    if let Some(ref p) = pwd {
                        self.last_known = Some(p.clone());
                        return Some(p.clone());
                    }
                    // Initial None value — keep waiting
                }
                Err(_) => return None,
            }
        }
    }

    /// Waits for the next PWD change event (with transition info).
    ///
    /// Returns a `PwdChangeEvent` containing both previous and current paths.
    /// Returns `None` if the PTY session has ended.
    pub async fn next_event(&mut self) -> Option<PwdChangeEvent> {
        let previous = self.last_known.clone();

        self.changed()
            .await
            .map(|(current, timestamp)| PwdChangeEvent {
                previous: previous.map(|(p, _)| p),
                current,
                timestamp,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_osc7_bel_terminated() {
        let mut parser = OscParser::new(2048);

        let paths = parser.feed(b"\x1b]7;file://hostname/home/user\x07");
        assert_eq!(paths, vec!["/home/user"]);
    }

    #[test]
    fn test_parser_osc7_st_terminated() {
        let mut parser = OscParser::new(2048);

        let paths = parser.feed(b"\x1b]7;file://hostname/home/user\x1b\\");
        assert_eq!(paths, vec!["/home/user"]);
    }

    #[test]
    fn test_parser_osc7_empty_hostname() {
        let mut parser = OscParser::new(2048);

        let paths = parser.feed(b"\x1b]7;file:///tmp/dir\x07");
        assert_eq!(paths, vec!["/tmp/dir"]);
    }

    #[test]
    fn test_parser_osc633() {
        let mut parser = OscParser::new(2048);

        let paths = parser.feed(b"\x1b]633;P;Cwd=/home/user/projects\x07");
        assert_eq!(paths, vec!["/home/user/projects"]);
    }

    #[test]
    fn test_parser_osc633_st_terminated() {
        let mut parser = OscParser::new(2048);

        let paths = parser.feed(b"\x1b]633;P;Cwd=/var/log\x1b\\");
        assert_eq!(paths, vec!["/var/log"]);
    }

    #[test]
    fn test_parser_split_sequences() {
        let mut parser = OscParser::new(2048);

        let paths = parser.feed(b"\x1b]7;file://");
        assert_eq!(paths, Vec::<String>::new());

        let paths = parser.feed(b"host/home");
        assert_eq!(paths, Vec::<String>::new());

        let paths = parser.feed(b"/user\x07");
        assert_eq!(paths, vec!["/home/user"]);
    }

    #[test]
    fn test_parser_mixed_content() {
        let mut parser = OscParser::new(2048);

        let paths = parser.feed(b"prompt$ \x1b]7;file://host/home/user\x07some output after");
        assert_eq!(paths, vec!["/home/user"]);
    }

    #[test]
    fn test_parser_multiple_sequences() {
        let mut parser = OscParser::new(2048);

        let paths = parser.feed(b"\x1b]7;file://h/home/a\x07text\x1b]7;file://h/home/b\x07");
        assert_eq!(paths, vec!["/home/a", "/home/b"]);
    }

    #[test]
    fn test_parser_invalid_osc() {
        let mut parser = OscParser::new(2048);

        // OSC with unrecognized code
        let paths = parser.feed(b"\x1b]99;something\x07");
        assert_eq!(paths, Vec::<String>::new());

        // Not a file:// URL
        let paths = parser.feed(b"\x1b]7;http://example.com\x07");
        assert_eq!(paths, Vec::<String>::new());
    }

    #[test]
    fn test_parser_buffer_overflow() {
        let mut parser = OscParser::new(64);

        // Payload exceeds buffer
        let long_path = "a".repeat(100);
        let seq = format!("\x1b]7;file://host/{}\x07", long_path);
        let paths = parser.feed(seq.as_bytes());
        assert_eq!(paths, Vec::<String>::new());

        // Parser should recover after overflow
        let paths = parser.feed(b"\x1b]7;file://h/ok\x07");
        assert_eq!(paths, vec!["/ok"]);
    }

    #[test]
    fn test_parser_percent_decoding() {
        let mut parser = OscParser::new(2048);

        let paths = parser.feed(b"\x1b]7;file://host/home/user/my%20project\x07");
        assert_eq!(paths, vec!["/home/user/my project"]);

        let paths = parser.feed(b"\x1b]7;file://host/tmp/%E2%9C%93\x07");
        assert_eq!(paths, vec!["/tmp/\u{2713}"]);
    }

    #[test]
    fn test_parser_percent_decoding_osc633() {
        let mut parser = OscParser::new(2048);

        let paths = parser.feed(b"\x1b]633;P;Cwd=/home/user/my%20dir\x07");
        assert_eq!(paths, vec!["/home/user/my dir"]);
    }

    #[test]
    fn test_percent_decode_function() {
        assert_eq!(percent_decode("/home/user"), "/home/user");
        assert_eq!(percent_decode("/home/user%20name"), "/home/user name");
        assert_eq!(percent_decode("%2Ftmp%2Ftest"), "/tmp/test");
        assert_eq!(percent_decode("no%zzencoding"), "no%zzencoding");
        assert_eq!(percent_decode("trailing%2"), "trailing%2");
        assert_eq!(percent_decode(""), "");
    }

    #[test]
    fn test_pwd_detection_disabled_noop() {
        let config = PwdDetectionConfig {
            enabled: false,
            buffer_size: 2048,
        };
        let detection = PwdDetection::new(config);

        detection.feed(b"\x1b]7;file://host/home/user\x07");
        assert!(detection.current_pwd().is_none());
    }

    #[test]
    fn test_pwd_detection_enabled() {
        let config = PwdDetectionConfig {
            enabled: true,
            buffer_size: 2048,
        };
        let detection = PwdDetection::new(config);

        detection.feed(b"\x1b]7;file://host/home/user\x07");
        assert_eq!(detection.current_pwd(), Some("/home/user".to_string()));
    }

    #[test]
    fn test_pwd_detection_deduplicates() {
        let config = PwdDetectionConfig {
            enabled: true,
            buffer_size: 2048,
        };
        let detection = PwdDetection::new(config);

        detection.feed(b"\x1b]7;file://host/home/user\x07");
        detection.feed(b"\x1b]7;file://host/home/user\x07");
        assert_eq!(detection.current_pwd(), Some("/home/user".to_string()));
    }

    #[test]
    fn test_pwd_detection_config_default() {
        let config = PwdDetectionConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.buffer_size, 2048);
    }

    #[tokio::test]
    async fn test_pwd_watcher_changed() {
        let (tx, rx) = watch::channel(None);
        let mut watcher = PwdWatcher {
            inner: rx,
            last_known: None,
        };

        tokio::spawn(async move {
            tx.send(Some(("/home/user".to_string(), Instant::now())))
                .unwrap();
        });

        let pwd = watcher.changed().await;
        assert_eq!(pwd.map(|(s, _)| s), Some("/home/user".to_string()));
    }

    #[tokio::test]
    async fn test_pwd_watcher_next_event() {
        let (tx, rx) = watch::channel(None);
        let mut watcher = PwdWatcher {
            inner: rx,
            last_known: None,
        };

        tokio::spawn(async move {
            tx.send(Some(("/home/user".to_string(), Instant::now())))
                .unwrap();
        });

        let event = watcher.next_event().await;
        assert!(event.is_some());
        let event = event.unwrap();
        assert_eq!(event.previous, None);
        assert_eq!(event.current, "/home/user");
    }

    #[tokio::test]
    async fn test_pwd_watcher_next_event_with_previous() {
        let (tx, rx) = watch::channel(Some(("/old/path".to_string(), Instant::now())));
        let mut watcher = PwdWatcher {
            inner: rx,
            last_known: Some(("/old/path".to_string(), Instant::now())),
        };

        tokio::spawn(async move {
            tx.send(Some(("/new/path".to_string(), Instant::now())))
                .unwrap();
        });

        let event = watcher.next_event().await;
        assert!(event.is_some());
        let event = event.unwrap();
        assert_eq!(event.previous, Some("/old/path".to_string()));
        assert_eq!(event.current, "/new/path");
    }

    #[tokio::test]
    async fn test_multiple_pwd_watchers() {
        let (tx, rx) = watch::channel(None);
        let watcher1 = PwdWatcher {
            inner: rx.clone(),
            last_known: None,
        };
        let watcher2 = PwdWatcher {
            inner: rx,
            last_known: None,
        };

        tx.send(Some(("/home/user".to_string(), Instant::now())))
            .unwrap();

        assert_eq!(watcher1.current(), Some("/home/user".to_string()));
        assert_eq!(watcher2.current(), Some("/home/user".to_string()));
    }

    #[tokio::test]
    async fn test_pwd_watcher_session_ended() {
        let (tx, rx) = watch::channel(None);
        let mut watcher = PwdWatcher {
            inner: rx,
            last_known: None,
        };

        drop(tx);

        let pwd = watcher.changed().await;
        assert_eq!(pwd, None);
    }
}
