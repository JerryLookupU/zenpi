#![cfg(feature = "remote")]

use std::io::{self, Cursor, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};
use zenpi::remote::frame::{FrameError, read_frame, write_frame};
use zenpi::remote::protocol::{Channel, Envelope, MAX_CONTROL_BYTES, MAX_FRAME_BYTES, Message};

fn hello() -> Envelope {
    Envelope::new(
        Channel::Control,
        Message::Hello {
            supported_versions: vec![1],
        },
    )
}

#[test]
fn network_order_roundtrip_and_multiple_frames() {
    let mut bytes = Vec::new();
    write_frame(&mut bytes, &hello()).unwrap();
    assert_eq!(
        u32::from_be_bytes(bytes[..4].try_into().unwrap()) as usize,
        bytes.len() - 4
    );
    write_frame(&mut bytes, &hello()).unwrap();
    let mut cursor = Cursor::new(bytes);
    assert_eq!(read_frame(&mut cursor).unwrap(), Some(hello()));
    assert_eq!(read_frame(&mut cursor).unwrap(), Some(hello()));
    assert_eq!(read_frame(&mut cursor).unwrap(), None);
}

struct Fragmented<R> {
    inner: R,
    interrupt: bool,
}
impl<R: Read> Read for Fragmented<R> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.interrupt = !self.interrupt;
        if self.interrupt {
            return Err(io::ErrorKind::Interrupted.into());
        }
        let length = bytes.len().min(1);
        self.inner.read(&mut bytes[..length])
    }
}

#[test]
fn fragmented_reads_and_interruptions_are_supported() {
    let mut bytes = Vec::new();
    write_frame(&mut bytes, &hello()).unwrap();
    let mut reader = Fragmented {
        inner: Cursor::new(bytes),
        interrupt: false,
    };
    assert_eq!(read_frame(&mut reader).unwrap(), Some(hello()));
}

#[test]
fn every_partial_frame_is_truncated_not_clean_eof() {
    let mut bytes = Vec::new();
    write_frame(&mut bytes, &hello()).unwrap();
    assert_eq!(read_frame(&mut Cursor::new(&bytes[..0])).unwrap(), None);
    for length in 1..bytes.len() {
        assert!(
            matches!(
                read_frame(&mut Cursor::new(&bytes[..length])),
                Err(FrameError::Truncated)
            ),
            "{length}"
        );
    }
}

#[test]
fn length_is_rejected_before_body_read_or_allocation() {
    struct PrefixOnly {
        prefix: Cursor<[u8; 4]>,
    }
    impl Read for PrefixOnly {
        fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            assert!(self.prefix.position() < 4, "must not read the body");
            self.prefix.read(bytes)
        }
    }
    for length in [0, MAX_FRAME_BYTES as u32 + 1, u32::MAX] {
        let mut reader = PrefixOnly {
            prefix: Cursor::new(length.to_be_bytes()),
        };
        assert!(
            matches!(read_frame(&mut reader), Err(FrameError::Length(value)) if value == length)
        );
    }
}

#[test]
fn malformed_json_utf8_and_trailing_data_are_refused() {
    for body in [b"{bad}".as_slice(), &[0xff], b"{}{}", b"null"] {
        let mut bytes = (body.len() as u32).to_be_bytes().to_vec();
        bytes.extend_from_slice(body);
        assert!(matches!(
            read_frame(&mut Cursor::new(bytes)),
            Err(FrameError::Protocol(_))
        ));
    }
}

#[test]
fn invalid_or_oversized_output_emits_no_prefix() {
    let mut envelope = hello();
    envelope.schema_version = 99;
    let mut bytes = Vec::new();
    assert!(write_frame(&mut bytes, &envelope).is_err());
    assert!(bytes.is_empty());
    envelope.schema_version = 1;
    envelope.payload = Message::Request {
        operation: zenpi::remote::protocol::Operation::Inspect,
        arguments: serde_json::json!({"text": "x".repeat(MAX_CONTROL_BYTES)}),
    };
    envelope.id = Some("r1".into());
    assert!(write_frame(&mut bytes, &envelope).is_err());
    assert!(bytes.is_empty());
}

#[test]
fn exact_channel_byte_limit_is_accepted_and_one_more_is_rejected() {
    let mut envelope = Envelope::new(
        Channel::Control,
        Message::Request {
            operation: zenpi::remote::protocol::Operation::Inspect,
            arguments: serde_json::json!({"text": ""}),
        },
    );
    envelope.id = Some("r1".into());
    let overhead = serde_json::to_vec(&envelope).unwrap().len();
    if let Message::Request { arguments, .. } = &mut envelope.payload {
        arguments["text"] = serde_json::json!("x".repeat(MAX_CONTROL_BYTES - overhead));
    }
    let mut bytes = Vec::new();
    write_frame(&mut bytes, &envelope).unwrap();
    assert_eq!(bytes.len(), MAX_CONTROL_BYTES + 4);
    assert_eq!(
        read_frame(&mut Cursor::new(bytes)).unwrap(),
        Some(envelope.clone())
    );
    if let Message::Request { arguments, .. } = &mut envelope.payload {
        arguments["text"] = serde_json::json!("x".repeat(MAX_CONTROL_BYTES - overhead + 1));
    }
    let mut bytes = Vec::new();
    assert!(write_frame(&mut bytes, &envelope).is_err());
    assert!(bytes.is_empty());
}

#[test]
fn partial_write_and_flush_failures_are_not_reported_as_success() {
    struct Broken {
        left: usize,
    }
    impl Write for Broken {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.left == 0 {
                return Err(io::ErrorKind::BrokenPipe.into());
            }
            let length = self.left.min(bytes.len());
            self.left -= length;
            Ok(length)
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::ErrorKind::BrokenPipe.into())
        }
    }
    for left in [0, 2, 5, usize::MAX] {
        assert!(matches!(
            write_frame(&mut Broken { left }, &hello()),
            Err(FrameError::Io(_))
        ));
    }
}

#[test]
fn injected_deadlines_at_every_prefix_and_body_offset_are_terminal_not_eof() {
    struct Deadline {
        bytes: Vec<u8>,
        position: usize,
        stop: usize,
    }
    impl Read for Deadline {
        fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
            if self.position >= self.stop {
                return Err(io::ErrorKind::TimedOut.into());
            }
            let count = output.len().min(self.stop - self.position);
            output[..count].copy_from_slice(&self.bytes[self.position..self.position + count]);
            self.position += count;
            Ok(count)
        }
    }
    let mut bytes = Vec::new();
    write_frame(&mut bytes, &hello()).unwrap();
    for stop in 0..bytes.len() {
        let mut input = Deadline {
            bytes: bytes.clone(),
            position: 0,
            stop,
        };
        assert!(
            matches!(read_frame(&mut input), Err(FrameError::Io(error)) if error.kind() == io::ErrorKind::TimedOut),
            "{stop}"
        );
    }
}

#[test]
fn fragmented_multi_frame_stream_preserves_boundaries_and_flush_count() {
    struct FragmentedOutput {
        bytes: Vec<u8>,
        chunk: usize,
        flushes: usize,
    }
    impl Write for FragmentedOutput {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            let count = bytes.len().min(self.chunk);
            self.bytes.extend_from_slice(&bytes[..count]);
            Ok(count)
        }
        fn flush(&mut self) -> io::Result<()> {
            self.flushes += 1;
            Ok(())
        }
    }
    for chunk in [1, 2, 3, 4, 7, 31, 127] {
        let mut output = FragmentedOutput {
            bytes: Vec::new(),
            chunk,
            flushes: 0,
        };
        for _ in 0..16 {
            write_frame(&mut output, &hello()).unwrap();
        }
        assert_eq!(output.flushes, 16);
        let mut input = Fragmented {
            inner: Cursor::new(output.bytes),
            interrupt: false,
        };
        for _ in 0..16 {
            assert_eq!(read_frame(&mut input).unwrap(), Some(hello()));
        }
        assert!(read_frame(&mut input).unwrap().is_none());
    }
}

/// Requires an OS that permits TCP loopback sockets; deterministic frame
/// faults remain in the default offline suite.
#[test]
#[ignore = "requires loopback socket permission"]
fn real_loopback_timeout_remains_a_terminal_io_error() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let _peer = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (mut stream, _) = listener.accept().unwrap();
    stream
        .set_read_timeout(Some(Duration::from_millis(50)))
        .unwrap();
    let started = Instant::now();
    match read_frame(&mut stream) {
        Err(FrameError::Io(error)) => assert!(matches!(
            error.kind(),
            io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
        )),
        other => panic!("expected timeout, got {other:?}"),
    }
    assert!(started.elapsed() < Duration::from_secs(3));
}
