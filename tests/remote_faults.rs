#![cfg(feature = "remote")]
//! Deterministic transport faults; real socket/TLS/address-change gates are separate.

use std::io::{self, Cursor, Read, Write};
use zenpi::remote::{
    frame::{FrameError, read_frame, write_frame},
    protocol::{Channel, Envelope, Message},
};

fn hello() -> Envelope {
    Envelope::new(
        Channel::Control,
        Message::Hello {
            supported_versions: vec![1],
        },
    )
}

struct FaultReader {
    bytes: Cursor<Vec<u8>>,
    at: u64,
    kind: io::ErrorKind,
}

impl Read for FaultReader {
    fn read(&mut self, target: &mut [u8]) -> io::Result<usize> {
        if self.bytes.position() >= self.at {
            return Err(self.kind.into());
        }
        let count = target.len().min((self.at - self.bytes.position()) as usize);
        self.bytes.read(&mut target[..count])
    }
}

#[test]
fn timeout_reset_and_half_read_faults_preserve_terminal_error() {
    let mut bytes = Vec::new();
    write_frame(&mut bytes, &hello()).unwrap();
    for at in [0, 1, 3, 4, bytes.len() as u64 - 1] {
        for kind in [
            io::ErrorKind::TimedOut,
            io::ErrorKind::ConnectionReset,
            io::ErrorKind::WouldBlock,
        ] {
            let mut reader = FaultReader {
                bytes: Cursor::new(bytes.clone()),
                at,
                kind,
            };
            assert!(
                matches!(read_frame(&mut reader), Err(FrameError::Io(error)) if error.kind() == kind)
            );
        }
    }
}

#[test]
fn clean_eof_after_complete_frame_is_distinct_from_all_partial_eof_positions() {
    let mut bytes = Vec::new();
    write_frame(&mut bytes, &hello()).unwrap();
    let mut complete = Cursor::new(bytes.clone());
    assert_eq!(read_frame(&mut complete).unwrap(), Some(hello()));
    assert!(read_frame(&mut complete).unwrap().is_none());
    for end in 1..bytes.len() {
        assert!(matches!(
            read_frame(&mut Cursor::new(&bytes[..end])),
            Err(FrameError::Truncated)
        ));
    }
}

#[test]
fn partial_output_timeout_and_zero_progress_are_not_commit_receipts() {
    struct Writer {
        remaining: usize,
        kind: Option<io::ErrorKind>,
    }
    impl Write for Writer {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.remaining == 0 {
                return self.kind.map_or(Ok(0), |kind| Err(kind.into()));
            }
            let count = self.remaining.min(bytes.len());
            self.remaining -= count;
            Ok(count)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    for remaining in [0, 2, 4, 7] {
        for kind in [
            None,
            Some(io::ErrorKind::TimedOut),
            Some(io::ErrorKind::BrokenPipe),
        ] {
            assert!(matches!(
                write_frame(&mut Writer { remaining, kind }, &hello()),
                Err(FrameError::Io(_))
            ));
        }
    }
}
