//! Keys read from the popup's stdin.

use std::io::Read;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::Duration;

const ESC: u8 = 0x1b;

/// How long a lone Esc byte waits for the rest of an escape sequence.
const ESC_WAIT: Duration = Duration::from_millis(50);

/// One key press.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// Left arrow.
    Left,
    /// Right arrow.
    Right,
    /// Up arrow.
    Up,
    /// Down arrow.
    Down,
    /// Enter.
    Enter,
    /// Esc on its own.
    Esc,
    /// Backspace.
    Backspace,
    /// A digit key, 0 to 9.
    Digit(u8),
    /// Another printable ASCII key.
    Char(char),
    /// Anything the popup does not use.
    Other,
}

/// What the front of a byte buffer holds.
#[derive(Debug, PartialEq, Eq)]
enum Parsed {
    /// A key and the number of bytes it took.
    Key(Key, usize),
    /// The start of an escape sequence whose end has not arrived.
    Incomplete,
}

/// Reads the front key of `bytes`, which must not be empty.
fn parse(bytes: &[u8]) -> Parsed {
    let key = |key, len| Parsed::Key(key, len);
    match bytes[0] {
        ESC => parse_escape(bytes),
        b'\r' | b'\n' => key(Key::Enter, 1),
        0x7f | 0x08 => key(Key::Backspace, 1),
        d @ b'0'..=b'9' => key(Key::Digit(d - b'0'), 1),
        c @ 0x20..=0x7e => key(Key::Char(char::from(c)), 1),
        _ => key(Key::Other, 1),
    }
}

fn arrow(final_byte: u8) -> Key {
    match final_byte {
        b'A' => Key::Up,
        b'B' => Key::Down,
        b'C' => Key::Right,
        b'D' => Key::Left,
        _ => Key::Other,
    }
}

fn parse_escape(bytes: &[u8]) -> Parsed {
    match bytes.get(1) {
        None => Parsed::Incomplete,
        Some(b'[') => {
            // Parameter and intermediate bytes, then one final byte; mouse
            // reports and unknown sequences end up as `Other`.
            let end = bytes[2..]
                .iter()
                .position(|b| (0x40..=0x7e).contains(b))
                .map(|i| i + 2);
            match end {
                None => Parsed::Incomplete,
                Some(i) if i == 2 => Parsed::Key(arrow(bytes[i]), 3),
                Some(i) => Parsed::Key(Key::Other, i + 1),
            }
        }
        Some(b'O') => bytes
            .get(2)
            .map_or(Parsed::Incomplete, |&b| Parsed::Key(arrow(b), 3)),
        Some(_) => Parsed::Key(Key::Other, 2),
    }
}

/// Turns the bytes arriving on stdin into keys.
///
/// A thread does the blocking read so a lone Esc can tell itself from the
/// start of an arrow key by waiting briefly for what follows.
#[derive(Debug)]
pub struct KeyReader {
    chunks: Receiver<Vec<u8>>,
    pending: Vec<u8>,
}

impl KeyReader {
    /// Starts reading stdin.
    #[must_use]
    pub fn from_stdin() -> Self {
        let (tx, chunks) = mpsc::channel();
        thread::spawn(move || {
            let mut stdin = std::io::stdin().lock();
            let mut buf = [0u8; 256];
            while let Ok(n) = stdin.read(&mut buf) {
                if n == 0 || tx.send(buf[..n].to_vec()).is_err() {
                    break;
                }
            }
        });
        Self {
            chunks,
            pending: Vec::new(),
        }
    }

    /// The next key, waiting for it; `None` once stdin has reached EOF and
    /// every byte read has been returned.
    pub fn next_key(&mut self) -> Option<Key> {
        loop {
            if self.pending.is_empty() {
                let chunk = self.chunks.recv().ok()?;
                self.pending.extend(chunk);
                continue;
            }
            match parse(&self.pending) {
                Parsed::Key(key, len) => {
                    self.pending.drain(..len);
                    return Some(key);
                }
                Parsed::Incomplete => match self.chunks.recv_timeout(ESC_WAIT) {
                    Ok(chunk) => self.pending.extend(chunk),
                    Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => {
                        let lone_esc = self.pending == [ESC];
                        self.pending.clear();
                        return Some(if lone_esc { Key::Esc } else { Key::Other });
                    }
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrows_and_plain_keys() {
        assert_eq!(parse(b"\x1b[D"), Parsed::Key(Key::Left, 3));
        assert_eq!(parse(b"\x1b[C"), Parsed::Key(Key::Right, 3));
        assert_eq!(parse(b"\x1b[A"), Parsed::Key(Key::Up, 3));
        assert_eq!(parse(b"\x1b[B"), Parsed::Key(Key::Down, 3));
        assert_eq!(parse(b"\x1bOA"), Parsed::Key(Key::Up, 3));
        assert_eq!(parse(b"\r"), Parsed::Key(Key::Enter, 1));
        assert_eq!(parse(b"\x7f"), Parsed::Key(Key::Backspace, 1));
        assert_eq!(parse(b"7"), Parsed::Key(Key::Digit(7), 1));
        assert_eq!(parse(b"p"), Parsed::Key(Key::Char('p'), 1));
    }

    #[test]
    fn partial_escapes_wait_for_more() {
        assert_eq!(parse(b"\x1b"), Parsed::Incomplete);
        assert_eq!(parse(b"\x1b["), Parsed::Incomplete);
        assert_eq!(parse(b"\x1b[<0;3;4"), Parsed::Incomplete);
    }

    #[test]
    fn mouse_reports_are_one_unused_key() {
        assert_eq!(parse(b"\x1b[<0;3;4m40"), Parsed::Key(Key::Other, 9));
    }
}
