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
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;
use tokio::sync::watch;

/// Represents the current PTY screen buffer mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtyMode {
    /// Standard/normal screen buffer (scrollback enabled)
    Standard,
    /// Alternate screen buffer (no scrollback, full-screen apps)
    Alternate,
}

impl PtyMode {
    /// Returns true if this is the alternate buffer mode.
    pub fn is_alternate(&self) -> bool {
        matches!(self, PtyMode::Alternate)
    }

    /// Returns true if this is the standard buffer mode.
    pub fn is_standard(&self) -> bool {
        matches!(self, PtyMode::Standard)
    }
}

/// A mode change event with old and new states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModeChangeEvent {
    /// The previous mode before the change.
    pub previous: PtyMode,
    /// The new mode after the change.
    pub current: PtyMode,
    /// Timestamp of when the change was detected.
    pub timestamp: Instant,
}

/// Configuration for mode detection behavior.
#[derive(Debug, Clone)]
pub struct ModeDetectionConfig {
    /// Whether to enable mode detection (default: false)
    pub enabled: bool,
    /// Buffer size for sequence parsing (default: 256 bytes)
    pub buffer_size: usize,
}

impl Default for ModeDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            buffer_size: 256,
        }
    }
}

/// Events detected by the escape sequence parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SequenceEvent {
    EnterAltMode,
    ExitAltMode,
}

enum ParserState {
    Normal,
    Escape,
    Csi,
    ModeSequence,
}

/// Parser for ANSI escape sequences that detects PTY mode changes.
///
/// This parser identifies sequences like `\x1b[?1049h` and `\x1b[?1049l`
/// which indicate transitions between standard and alternate screen buffers.
struct EscapeSequenceParser {
    buffer: Vec<u8>,
    state: ParserState,
    max_buffer_size: usize,
}

impl EscapeSequenceParser {
    /// Creates a new parser with the specified maximum buffer size.
    fn new(buffer_size: usize) -> Self {
        let buffer_size = buffer_size.max(8);
        Self {
            buffer: Vec::with_capacity(buffer_size),
            state: ParserState::Normal,
            max_buffer_size: buffer_size,
        }
    }

    /// Feeds raw bytes to the parser and returns any detected mode change events.
    fn feed(&mut self, data: &[u8]) -> Vec<SequenceEvent> {
        let mut events = Vec::new();

        for &byte in data {
            match self.state {
                ParserState::Normal => {
                    if byte == 0x1b {
                        self.state = ParserState::Escape;
                        self.buffer.clear();
                        self.buffer.push(byte);
                    }
                }
                ParserState::Escape => {
                    if self.buffer.len() < self.max_buffer_size {
                        self.buffer.push(byte);
                    }
                    if byte == b'[' {
                        self.state = ParserState::Csi;
                    } else {
                        self.state = ParserState::Normal;
                        self.buffer.clear();
                    }
                }
                ParserState::Csi => {
                    if self.buffer.len() < self.max_buffer_size {
                        self.buffer.push(byte);
                    }
                    if byte == b'?' {
                        self.state = ParserState::ModeSequence;
                    } else if byte.is_ascii_digit() || byte == b';' {
                    } else {
                        self.state = ParserState::Normal;
                        self.buffer.clear();
                    }
                }
                ParserState::ModeSequence => {
                    if self.buffer.len() >= self.max_buffer_size {
                        self.reset();
                    } else {
                        self.buffer.push(byte);
                        if byte == b'h' {
                            if self.is_alt_mode_sequence() {
                                events.push(SequenceEvent::EnterAltMode);
                            }
                            self.reset();
                        } else if byte == b'l' {
                            if self.is_alt_mode_sequence() {
                                events.push(SequenceEvent::ExitAltMode);
                            }
                            self.reset();
                        } else if !byte.is_ascii_digit() {
                            self.reset();
                        }
                    }
                }
            }
        }

        events
    }

    /// Checks if the current buffer contains a recognized alternate mode sequence.
    fn is_alt_mode_sequence(&self) -> bool {
        if self.buffer.len() < 4 {
            return false;
        }
        let seq = &self.buffer[2..];
        seq == b"?47h" || seq == b"?47l" || seq == b"?1049h" || seq == b"?1049l"
    }

    /// Resets the parser state and clears the buffer.
    fn reset(&mut self) {
        self.state = ParserState::Normal;
        self.buffer.clear();
    }
}

/// Internal PTY mode detection state for tracking screen buffer changes.
///
/// Manages the escape sequence parser and broadcasts mode changes to watchers.
pub(crate) struct ModeDetection {
    current_mode: Arc<Mutex<PtyMode>>,
    parser: Mutex<EscapeSequenceParser>,
    event_tx: watch::Sender<PtyMode>,
    event_rx: watch::Receiver<PtyMode>,
    enabled: bool,
}

impl ModeDetection {
    /// Creates a new mode detector with the given configuration.
    pub(crate) fn new(config: ModeDetectionConfig) -> Self {
        let (tx, rx) = watch::channel(PtyMode::Standard);
        Self {
            current_mode: Arc::new(Mutex::new(PtyMode::Standard)),
            parser: Mutex::new(EscapeSequenceParser::new(config.buffer_size)),
            event_tx: tx,
            event_rx: rx,
            enabled: config.enabled,
        }
    }

    /// Returns the current PTY mode without blocking.
    pub(crate) fn current_mode(&self) -> PtyMode {
        *self.current_mode.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Updates the current mode and notifies all watchers.
    pub(crate) fn update_mode(&self, new_mode: PtyMode) {
        let mut current = self.current_mode.lock().unwrap_or_else(|e| e.into_inner());
        if *current != new_mode {
            *current = new_mode;
            let _ = self.event_tx.send(new_mode);
        }
    }

    /// Feeds data to the escape sequence parser.
    ///
    /// If mode detection is disabled, this is a no-op.
    pub(crate) fn feed(&self, data: &[u8]) {
        if !self.enabled {
            return;
        }
        let mut parser = self.parser.lock().unwrap_or_else(|e| e.into_inner());
        let events = parser.feed(data);
        for event in events {
            match event {
                SequenceEvent::EnterAltMode => self.update_mode(PtyMode::Alternate),
                SequenceEvent::ExitAltMode => self.update_mode(PtyMode::Standard),
            }
        }
    }

    /// Creates a new watcher for observing mode changes.
    pub(crate) fn create_watcher(&self) -> Result<ModeWatcher> {
        Ok(ModeWatcher {
            inner: self.event_rx.clone(),
            last_known: *self.current_mode.lock().unwrap_or_else(|e| e.into_inner()),
        })
    }
}

/// An async-enabled watcher for PTY mode changes.
#[derive(Debug, Clone)]
pub struct ModeWatcher {
    inner: watch::Receiver<PtyMode>,
    last_known: PtyMode,
}

impl ModeWatcher {
    /// Returns the current mode without waiting.
    pub fn current(&self) -> PtyMode {
        *self.inner.borrow()
    }

    /// Waits for the mode to change and returns the new mode.
    ///
    /// This is an async method that can be awaited.
    /// Returns `None` if the PTY session has ended.
    pub async fn changed(&mut self) -> Option<PtyMode> {
        match self.inner.changed().await {
            Ok(()) => {
                let mode = *self.inner.borrow();
                self.last_known = mode;
                Some(mode)
            }
            Err(_) => None,
        }
    }

    /// Waits for the mode to change to a specific target mode.
    ///
    /// Returns immediately if already in that mode.
    /// Returns `None` if the PTY session ends before reaching target.
    pub async fn wait_for(&mut self, target: PtyMode) -> Option<PtyMode> {
        if self.current() == target {
            return Some(target);
        }

        while let Some(mode) = self.changed().await {
            if mode == target {
                return Some(target);
            }
        }
        None
    }

    /// Waits for the next mode change event (with transition info).
    ///
    /// Returns a `ModeChangeEvent` containing both previous and current modes.
    /// Returns `None` if the PTY session has ended.
    pub async fn next_event(&mut self) -> Option<ModeChangeEvent> {
        let previous = self.last_known;

        self.changed().await.map(|current| ModeChangeEvent {
            previous,
            current,
            timestamp: Instant::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_complete_sequences() {
        let mut parser = EscapeSequenceParser::new(256);

        let events = parser.feed(b"\x1b[?1049h");
        assert_eq!(events, vec![SequenceEvent::EnterAltMode]);

        let events = parser.feed(b"\x1b[?1049l");
        assert_eq!(events, vec![SequenceEvent::ExitAltMode]);

        let events = parser.feed(b"\x1b[?47h");
        assert_eq!(events, vec![SequenceEvent::EnterAltMode]);

        let events = parser.feed(b"\x1b[?47l");
        assert_eq!(events, vec![SequenceEvent::ExitAltMode]);
    }

    #[test]
    fn test_parser_split_sequences() {
        let mut parser = EscapeSequenceParser::new(256);

        let events = parser.feed(b"\x1b[?");
        assert_eq!(events, vec![]);

        let events = parser.feed(b"1049");
        assert_eq!(events, vec![]);

        let events = parser.feed(b"h");
        assert_eq!(events, vec![SequenceEvent::EnterAltMode]);
    }

    #[test]
    fn test_parser_mixed_content() {
        let mut parser = EscapeSequenceParser::new(256);

        let events = parser.feed(b"hello world\x1b[?1049hsome text");
        assert_eq!(events, vec![SequenceEvent::EnterAltMode]);

        let events = parser.feed(b"\x1b[?1049lmore text");
        assert_eq!(events, vec![SequenceEvent::ExitAltMode]);
    }

    #[test]
    fn test_parser_invalid_sequences() {
        let mut parser = EscapeSequenceParser::new(256);

        let events = parser.feed(b"\x1b[?1048h");
        assert_eq!(events, vec![]);

        let events = parser.feed(b"\x1b[J");
        assert_eq!(events, vec![]);

        let events = parser.feed(b"\x1bX");
        assert_eq!(events, vec![]);
    }

    #[test]
    fn test_parser_binary_data() {
        let mut parser = EscapeSequenceParser::new(256);

        let events = parser.feed(&[0x00, 0xff, 0x1b, 0x5b, 0x3f, 0x31, 0x30, 0x34, 0x39, 0x68]);
        assert_eq!(events, vec![SequenceEvent::EnterAltMode]);
    }

    #[tokio::test]
    async fn test_mode_watcher_changed() {
        let (tx, rx) = watch::channel(PtyMode::Standard);
        let mut watcher = ModeWatcher {
            inner: rx,
            last_known: PtyMode::Standard,
        };

        tokio::spawn(async move {
            tx.send(PtyMode::Alternate).unwrap();
        });

        let mode = watcher.changed().await;
        assert_eq!(mode, Some(PtyMode::Alternate));
    }

    #[tokio::test]
    async fn test_watcher_wait_for_immediate() {
        let (_tx, rx) = watch::channel(PtyMode::Alternate);
        let mut watcher = ModeWatcher {
            inner: rx,
            last_known: PtyMode::Standard,
        };

        let mode = watcher.wait_for(PtyMode::Alternate).await;
        assert_eq!(mode, Some(PtyMode::Alternate));
    }

    #[tokio::test]
    async fn test_watcher_wait_for_with_change() {
        let (tx, rx) = watch::channel(PtyMode::Standard);
        let mut watcher = ModeWatcher {
            inner: rx,
            last_known: PtyMode::Standard,
        };

        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            tx.send(PtyMode::Alternate).unwrap();
        });

        let mode = watcher.wait_for(PtyMode::Alternate).await;
        assert_eq!(mode, Some(PtyMode::Alternate));
    }

    #[tokio::test]
    async fn test_watcher_next_event() {
        let (tx, rx) = watch::channel(PtyMode::Standard);
        let mut watcher = ModeWatcher {
            inner: rx,
            last_known: PtyMode::Standard,
        };

        tokio::spawn(async move {
            tx.send(PtyMode::Alternate).unwrap();
        });

        let event = watcher.next_event().await;
        assert!(event.is_some());
        let event = event.unwrap();
        assert_eq!(event.previous, PtyMode::Standard);
        assert_eq!(event.current, PtyMode::Alternate);
    }

    #[tokio::test]
    async fn test_multiple_watchers() {
        let (tx, rx) = watch::channel(PtyMode::Standard);
        let watcher1 = ModeWatcher {
            inner: rx.clone(),
            last_known: PtyMode::Standard,
        };
        let watcher2 = ModeWatcher {
            inner: rx,
            last_known: PtyMode::Standard,
        };

        tx.send(PtyMode::Alternate).unwrap();

        assert_eq!(watcher1.current(), PtyMode::Alternate);
        assert_eq!(watcher2.current(), PtyMode::Alternate);
    }

    #[test]
    fn test_mode_methods() {
        assert!(PtyMode::Standard.is_standard());
        assert!(!PtyMode::Standard.is_alternate());
        assert!(PtyMode::Alternate.is_alternate());
        assert!(!PtyMode::Alternate.is_standard());
    }

    #[test]
    fn test_mode_detection_config_default() {
        let config = ModeDetectionConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.buffer_size, 256);
    }
}
