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

use parking_lot::Mutex;
use std::collections::VecDeque;
use std::time::Instant;
use tokio::sync::watch;

/// Configuration for PTY history capture.
///
/// Use the builder pattern to configure limits:
///
/// ```
/// use simple_ssh::PtyHistoryConfig;
///
/// let config = PtyHistoryConfig::new()
///     .lines(1000)
///     .memory("10MiB");
/// ```
#[derive(Debug, Clone)]
pub struct PtyHistoryConfig {
    pub(crate) enabled: bool,
    pub(crate) max_lines: usize,
    pub(crate) max_memory_bytes: usize,
}

impl Default for PtyHistoryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_lines: 1000,
            max_memory_bytes: 1024 * 1024, // 1MiB
        }
    }
}

impl PtyHistoryConfig {
    /// Creates a new history config with defaults.
    ///
    /// Defaults:
    /// - max_lines: 1000
    /// - max_memory: 1MiB
    pub fn new() -> Self {
        Self {
            enabled: true,
            ..Default::default()
        }
    }

    /// Sets the maximum number of lines to retain.
    ///
    /// When the limit is exceeded, oldest lines are evicted.
    pub fn lines(mut self, count: usize) -> Self {
        self.max_lines = count.max(1);
        self
    }

    /// Sets the maximum memory usage.
    ///
    /// Supports human-readable formats:
    /// - `"10MiB"`, `"10MB"`, `"10M"` - megabytes
    /// - `"1GiB"`, `"1GB"`, `"1G"` - gigabytes  
    /// - `"500KiB"`, `"500KB"`, `"500K"` - kilobytes
    /// - `"1048576"` - raw bytes
    ///
    /// # Panics
    ///
    /// Panics if the string cannot be parsed.
    pub fn memory(mut self, limit: &str) -> Self {
        self.max_memory_bytes = parse_memory_limit(limit);
        self
    }
}

/// Parses a memory limit string into bytes.
///
/// All suffixes are interpreted as binary units (powers of 1024), not decimal
/// (powers of 1000). This means "MB" means 1024*1024 bytes, not 1000*1000.
///
/// Supports:
/// - `"10MiB"`, `"10MB"`, `"10M"` → 10 * 1024 * 1024
/// - `"1GiB"`, `"1GB"`, `"1G"` → 1 * 1024 * 1024 * 1024
/// - `"500KiB"`, `"500KB"`, `"500K"` → 500 * 1024
/// - `"1048576"` → 1048576 (raw bytes)
///
/// # Rounding Behavior
///
/// The minimum enforced value is 1024 bytes. Any input below this threshold
/// will be rounded up to 1024. Decimal values are truncated to integers
/// during conversion.
fn parse_memory_limit(input: &str) -> usize {
    let input = input.trim();

    // Try to parse as raw number first
    if let Ok(bytes) = input.parse::<usize>() {
        return bytes.max(1024);
    }

    // Parse with suffix
    let mut num_str = String::new();
    let mut suffix = String::new();

    for ch in input.chars() {
        if ch.is_ascii_digit() || ch == '.' {
            num_str.push(ch);
        } else {
            suffix.push(ch.to_ascii_uppercase());
        }
    }

    let num: f64 = num_str.parse().expect("Invalid number in memory limit");
    let multiplier = match suffix.as_str() {
        "K" | "KB" | "KIB" => 1024usize,
        "M" | "MB" | "MIB" => 1024usize * 1024,
        "G" | "GB" | "GIB" => 1024usize * 1024 * 1024,
        _ => panic!("Unknown memory unit: {}", suffix),
    };

    ((num * multiplier as f64) as usize).max(1024)
}

/// A single history entry with timestamp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    /// The line content (ANSI escape sequences stripped).
    pub content: String,
    /// When this line was received.
    pub timestamp: Instant,
}

/// Internal history state for tracking PTY output.
///
/// Manages a circular buffer of lines with memory and line count limits.
pub(crate) struct PtyHistory {
    config: PtyHistoryConfig,
    entries: Mutex<VecDeque<HistoryEntry>>,
    current_memory: Mutex<usize>,
    event_tx: watch::Sender<(usize, String)>, // Sends (count, new_line)
    event_rx: watch::Receiver<(usize, String)>,
    pending: Mutex<String>,
}

impl PtyHistory {
    /// Creates a new history with the given configuration.
    pub(crate) fn new(config: PtyHistoryConfig) -> Self {
        let (tx, rx) = watch::channel((0, String::new()));
        Self {
            config,
            entries: Mutex::new(VecDeque::with_capacity(100)),
            current_memory: Mutex::new(0),
            event_tx: tx,
            event_rx: rx,
            pending: Mutex::new(String::new()),
        }
    }

    /// Returns the number of entries in history.
    pub(crate) fn len(&self) -> usize {
        self.entries.lock().len()
    }

    /// Feeds raw PTY data to the history buffer.
    ///
    /// Data is appended to an internal pending buffer and only split on
    /// newline boundaries. Complete lines are processed and added to the
    /// buffer (with eviction when limits are exceeded). Any trailing partial
    /// line is preserved in the pending buffer for subsequent feed() calls.
    pub(crate) fn feed(&self, data: &[u8]) {
        let mut pending = self.pending.lock();
        let text = String::from_utf8_lossy(data);
        pending.push_str(&text);

        let mut start = 0;
        while let Some(newline_pos) = pending[start..].find('\n') {
            let line_end = start + newline_pos;
            let line = &pending[start..line_end];

            if !line.is_empty() {
                let cleaned = strip_ansi_codes(line);
                let entry = HistoryEntry {
                    content: cleaned.clone(),
                    timestamp: Instant::now(),
                };

                self.add_entry(entry);
                let _ = self.event_tx.send((self.len(), cleaned));
            }

            start = line_end + 1;
        }

        if start > 0 {
            pending.drain(..start);
        }
    }

    /// Adds a single entry, handling eviction.
    fn add_entry(&self, entry: HistoryEntry) {
        let entry_size = entry.content.len();
        let mut entries = self.entries.lock();
        let mut memory = self.current_memory.lock();

        // Add new entry
        *memory += entry_size;
        entries.push_back(entry);

        // Evict old entries if needed
        while entries.len() > self.config.max_lines || *memory > self.config.max_memory_bytes {
            if let Some(old) = entries.pop_front() {
                *memory = memory.saturating_sub(old.content.len());
            } else {
                break;
            }
        }
    }

    /// Returns an iterator over all history entries.
    pub(crate) fn iter(&self) -> impl Iterator<Item = HistoryEntry> {
        self.entries.lock().clone().into_iter()
    }

    /// Creates a new watcher for observing history changes.
    pub(crate) fn create_watcher(&self) -> HistoryWatcher {
        HistoryWatcher {
            inner: self.event_rx.clone(),
            last_known_count: self.len(),
        }
    }
}

/// An async-enabled watcher for history changes.
///
/// Provides async access to new lines as they are added.
#[derive(Debug, Clone)]
pub struct HistoryWatcher {
    inner: watch::Receiver<(usize, String)>,
    last_known_count: usize,
}

impl HistoryWatcher {
    /// Returns the current number of history entries.
    pub fn current_count(&self) -> usize {
        self.inner.borrow().0
    }

    /// Waits for the history to change and returns the new (count, line).
    ///
    /// Returns `None` if the PTY session has ended.
    ///
    /// # Important Behavior
    ///
    /// The underlying watch channel (via `self.inner`) only retains the latest
    /// value. If multiple lines are added rapidly between calls, intermediate
    /// lines will be skipped and only the most recent line will be returned.
    /// For applications that must process every line, poll this method
    /// frequently or consider using an alternative API that buffers all
    /// pending lines.
    pub async fn changed(&mut self) -> Option<(usize, String)> {
        match self.inner.changed().await {
            Ok(()) => {
                let (count, line) = self.inner.borrow().clone();
                self.last_known_count = count;
                Some((count, line))
            }
            Err(_) => None,
        }
    }

    /// Waits for and returns new entries as they arrive.
    ///
    /// Returns `None` if the PTY session has ended.
    /// Returns `Some((count, line))` where `count` is the total entry count
    /// and `line` is the new line content.
    pub async fn wait_for_new(&mut self) -> Option<(usize, String)> {
        loop {
            match self.inner.changed().await {
                Ok(()) => {
                    let (count, line) = self.inner.borrow().clone();
                    if count > self.last_known_count {
                        self.last_known_count = count;
                        return Some((count, line));
                    }
                }
                Err(_) => return None,
            }
        }
    }
}

/// Strips ANSI escape sequences from text.
///
/// Removes all escape sequences (CSI and OSC) while preserving visible
/// characters.
///
/// Handles:
/// - CSI sequences: ESC '[' followed by parameters and a final letter
/// - OSC sequences: ESC ']' followed by payload terminated by BEL ('\x07')
///   or ESC '\' (two-byte ST sequence)
fn strip_ansi_codes(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            match chars.peek() {
                Some(&'[') => {
                    chars.next();
                    while let Some(&ch) = chars.peek() {
                        chars.next();
                        if ch.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
                Some(&']') => {
                    chars.next();
                    while let Some(&ch) = chars.peek() {
                        chars.next();
                        if ch == '\x07' {
                            break;
                        }
                        if ch == '\x1b' {
                            if chars.peek() == Some(&'\\') {
                                chars.next();
                            }
                            break;
                        }
                    }
                }
                _ => {}
            }
        } else {
            result.push(ch);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_config_default() {
        let config = PtyHistoryConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.max_lines, 1000);
        assert_eq!(config.max_memory_bytes, 1024 * 1024);
    }

    #[test]
    fn test_config_new() {
        let config = PtyHistoryConfig::new();
        assert!(config.enabled);
        assert_eq!(config.max_lines, 1000);
        assert_eq!(config.max_memory_bytes, 1024 * 1024);
    }

    #[test]
    fn test_config_builder() {
        let config = PtyHistoryConfig::new().lines(500).memory("10MiB");

        assert!(config.enabled);
        assert_eq!(config.max_lines, 500);
        assert_eq!(config.max_memory_bytes, 10 * 1024 * 1024);
    }

    #[test]
    fn test_config_lines_only() {
        let config = PtyHistoryConfig::new().lines(100);
        assert_eq!(config.max_lines, 100);
        assert_eq!(config.max_memory_bytes, 1024 * 1024); // Default
    }

    #[test]
    fn test_config_memory_only() {
        let config = PtyHistoryConfig::new().memory("5MiB");
        assert_eq!(config.max_lines, 1000); // Default
        assert_eq!(config.max_memory_bytes, 5 * 1024 * 1024);
    }

    #[test]
    fn test_config_lines_minimum() {
        let config = PtyHistoryConfig::new().lines(0);
        assert_eq!(config.max_lines, 1);
    }

    #[test]
    fn test_parse_memory_raw_bytes() {
        assert_eq!(parse_memory_limit("1048576"), 1048576);
    }

    #[test]
    fn test_parse_memory_kilobytes() {
        assert_eq!(parse_memory_limit("100K"), 100 * 1024);
        assert_eq!(parse_memory_limit("100KB"), 100 * 1024);
        assert_eq!(parse_memory_limit("100KiB"), 100 * 1024);
    }

    #[test]
    fn test_parse_memory_megabytes() {
        assert_eq!(parse_memory_limit("10M"), 10 * 1024 * 1024);
        assert_eq!(parse_memory_limit("10MB"), 10 * 1024 * 1024);
        assert_eq!(parse_memory_limit("10MiB"), 10 * 1024 * 1024);
    }

    #[test]
    fn test_parse_memory_gigabytes() {
        assert_eq!(parse_memory_limit("1G"), 1024 * 1024 * 1024);
        assert_eq!(parse_memory_limit("1GB"), 1024 * 1024 * 1024);
        assert_eq!(parse_memory_limit("1GiB"), 1024 * 1024 * 1024);
    }

    #[test]
    fn test_parse_memory_with_whitespace() {
        assert_eq!(parse_memory_limit("  10MiB  "), 10 * 1024 * 1024);
    }

    #[test]
    fn test_parse_memory_decimal() {
        assert_eq!(
            parse_memory_limit("1.5MiB"),
            (1.5 * 1024.0 * 1024.0) as usize
        );
    }

    #[test]
    fn test_parse_memory_minimum() {
        assert_eq!(parse_memory_limit("100"), 1024); // Minimum enforced
    }

    #[test]
    #[should_panic(expected = "Invalid number")]
    fn test_parse_memory_invalid_number() {
        parse_memory_limit("abc");
    }

    #[test]
    #[should_panic(expected = "Unknown memory unit")]
    fn test_parse_memory_invalid_unit() {
        parse_memory_limit("10TB");
    }

    #[test]
    fn test_history_entry() {
        let entry = HistoryEntry {
            content: "test line".to_string(),
            timestamp: Instant::now(),
        };
        assert_eq!(entry.content, "test line");
    }

    #[test]
    fn test_history_new() {
        let config = PtyHistoryConfig::new().lines(10);
        let history = PtyHistory::new(config);
        assert_eq!(history.len(), 0);
    }

    #[test]
    fn test_history_feed_single_line() {
        let config = PtyHistoryConfig::new().lines(10);
        let history = PtyHistory::new(config);

        history.feed(b"hello world\n");

        assert_eq!(history.len(), 1);
        let entries: Vec<_> = history.iter().collect();
        assert_eq!(entries[0].content, "hello world");
    }

    #[test]
    fn test_history_feed_multiple_lines() {
        let config = PtyHistoryConfig::new().lines(10);
        let history = PtyHistory::new(config);

        history.feed(b"line1\nline2\nline3\n");

        assert_eq!(history.len(), 3);
        let entries: Vec<_> = history.iter().collect();
        assert_eq!(entries[0].content, "line1");
        assert_eq!(entries[1].content, "line2");
        assert_eq!(entries[2].content, "line3");
    }

    #[test]
    fn test_history_feed_no_newline() {
        let config = PtyHistoryConfig::new().lines(10);
        let history = PtyHistory::new(config);

        history.feed(b"no newline");

        // Partial line without newline stays in pending buffer
        assert_eq!(history.len(), 0);

        // Now add the newline to complete the line
        history.feed(b"\n");

        assert_eq!(history.len(), 1);
        let entries: Vec<_> = history.iter().collect();
        assert_eq!(entries[0].content, "no newline");
    }

    #[test]
    fn test_history_feed_partial_across_feeds() {
        let config = PtyHistoryConfig::new().lines(10);
        let history = PtyHistory::new(config);

        history.feed(b"hello ");
        assert_eq!(history.len(), 0);

        history.feed(b"world\n");
        assert_eq!(history.len(), 1);

        let entries: Vec<_> = history.iter().collect();
        assert_eq!(entries[0].content, "hello world");
    }

    #[test]
    fn test_history_feed_empty_lines_filtered() {
        let config = PtyHistoryConfig::new().lines(10);
        let history = PtyHistory::new(config);

        history.feed(b"line1\n\n\nline2\n");

        assert_eq!(history.len(), 2);
        let entries: Vec<_> = history.iter().collect();
        assert_eq!(entries[0].content, "line1");
        assert_eq!(entries[1].content, "line2");
    }

    #[test]
    fn test_history_feed_with_ansi() {
        let config = PtyHistoryConfig::new().lines(10);
        let history = PtyHistory::new(config);

        // ANSI color codes
        history.feed(b"\x1b[31mred text\x1b[0m\n");

        let entries: Vec<_> = history.iter().collect();
        assert_eq!(entries[0].content, "red text");
    }

    #[test]
    fn test_history_line_limit_eviction() {
        let config = PtyHistoryConfig::new().lines(3);
        let history = PtyHistory::new(config);

        history.feed(b"line1\nline2\nline3\n");
        assert_eq!(history.len(), 3);

        history.feed(b"line4\n");
        assert_eq!(history.len(), 3); // Still 3

        let entries: Vec<_> = history.iter().collect();
        assert_eq!(entries[0].content, "line2"); // line1 evicted
        assert_eq!(entries[1].content, "line3");
        assert_eq!(entries[2].content, "line4");
    }

    #[test]
    fn test_history_memory_limit_eviction() {
        // Each line is ~10 bytes, limit to 2500 bytes (above the 1024 minimum)
        let config = PtyHistoryConfig::new().lines(100).memory("2500");
        let history = PtyHistory::new(config);

        // Feed lines that total ~250 bytes each (with overhead)
        // After 25 lines we'll hit the limit
        for i in 0..30 {
            let line = format!("line{:04}\n", i); // 9 bytes per line
            history.feed(line.as_bytes());
        }

        // Should have evicted old entries to stay under memory limit
        // 2500 bytes / ~9 bytes per line = ~277 lines max, but we only fed 30
        // Actually the memory is just content.len(), so 8 chars = 8 bytes
        // 30 * 8 = 240 bytes which is well under 2500
        // Let me recalculate: we need to feed enough to exceed 2500 bytes
        // 2500 / 8 = 312.5, so we need ~313 lines to trigger eviction

        // Actually let me simplify - use larger lines
        let config2 = PtyHistoryConfig::new().lines(1000).memory("2048"); // 2KB limit
        let history2 = PtyHistory::new(config2);

        // Each line is ~100 bytes, after 20 lines we hit ~2000 bytes
        // The 21st line should trigger eviction
        for i in 0..25 {
            let line = format!("{:0100}\n", i); // 101 bytes per line (100 digits + newline)
            history2.feed(line.as_bytes());
        }

        // Should have around 20 entries (2000 bytes / 100 bytes per line)
        let count = history2.len();
        assert!(
            count >= 19 && count <= 21,
            "Expected ~20 entries for 2KB limit with 100B lines, got {}",
            count
        );
    }

    #[test]
    fn test_strip_ansi_codes() {
        assert_eq!(strip_ansi_codes("hello"), "hello");
        assert_eq!(strip_ansi_codes("\x1b[31mred\x1b[0m"), "red");
        assert_eq!(strip_ansi_codes("\x1b[1;31mbold red\x1b[0m"), "bold red");
        assert_eq!(strip_ansi_codes("\x1b[J"), "");
    }

    #[test]
    fn test_strip_ansi_mixed() {
        let input = "\x1b[32mgreen\x1b[0m normal \x1b[31mred\x1b[0m";
        assert_eq!(strip_ansi_codes(input), "green normal red");
    }

    #[test]
    fn test_strip_ansi_osc_bel_terminator() {
        assert_eq!(strip_ansi_codes("\x1b]0;title\x07"), "");
        assert_eq!(strip_ansi_codes("text\x1b]0;title\x07more"), "textmore");
    }

    #[test]
    fn test_strip_ansi_osc_st_terminator() {
        assert_eq!(strip_ansi_codes("\x1b]0;title\x1b\\"), "");
        assert_eq!(strip_ansi_codes("text\x1b]0;title\x1b\\more"), "textmore");
    }

    #[test]
    fn test_strip_ansi_osc_mixed_with_csi() {
        let input = "\x1b[31mred\x1b[0m \x1b]0;title\x07green\x1b[32m bright\x1b[0m";
        assert_eq!(strip_ansi_codes(input), "red green bright");
    }

    #[test]
    fn test_history_iter_empty() {
        let config = PtyHistoryConfig::new().lines(10);
        let history = PtyHistory::new(config);

        let count = history.iter().count();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_history_iter_order() {
        let config = PtyHistoryConfig::new().lines(10);
        let history = PtyHistory::new(config);

        history.feed(b"first\nsecond\nthird\n");

        let contents: Vec<_> = history.iter().map(|e| e.content).collect();
        assert_eq!(contents, vec!["first", "second", "third"]);
    }

    #[tokio::test]
    async fn test_history_watcher() {
        let config = PtyHistoryConfig::new().lines(10);
        let history = PtyHistory::new(config);
        let mut watcher = history.create_watcher();

        assert_eq!(watcher.current_count(), 0);

        // Spawn a task to feed data
        let history_clone = Arc::new(history);
        let history_feed = history_clone.clone();

        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            history_feed.feed(b"test line\n");
        });

        // Wait for change
        let result = watcher.changed().await;
        assert_eq!(result, Some((1, "test line".to_string())));
        assert_eq!(watcher.current_count(), 1);
    }

    #[tokio::test]
    async fn test_history_watcher_wait_for_new() {
        let config = PtyHistoryConfig::new().lines(10);
        let history = PtyHistory::new(config);
        let mut watcher = history.create_watcher();

        // Spawn task to add data after a delay
        let history_feed = Arc::new(history);
        let feed_clone = history_feed.clone();

        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            feed_clone.feed(b"new line\n");
        });

        // Wait for new entries - this should block until the feed happens
        let result = watcher.wait_for_new().await;
        assert_eq!(result, Some((1, "new line".to_string())));

        // Verify we got the content
        assert_eq!(history_feed.len(), 1);
    }

    #[tokio::test]
    async fn test_history_watcher_session_ended() {
        let config = PtyHistoryConfig::new().lines(10);
        let history = PtyHistory::new(config);
        let mut watcher = history.create_watcher();

        // Drop the sender to simulate session end
        drop(history);

        let count = watcher.changed().await;
        assert_eq!(count, None);
    }
}
