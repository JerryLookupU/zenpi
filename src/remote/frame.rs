//! Blocking length-prefixed JSON I/O for caller-owned streams.
//!
//! The listener must configure stream deadlines. A timeout, truncated frame,
//! invalid frame or partial write is terminal: discard the connection, never
//! retry this codec on the partially consumed stream.

use std::io::{self, Read, Write};

use thiserror::Error;

use super::protocol::{Envelope, MAX_FRAME_BYTES, ProtocolError};

#[derive(Debug, Error)]
pub enum FrameError {
    #[error("remote stream I/O: {0}")]
    Io(#[from] io::Error),
    #[error("truncated remote frame")]
    Truncated,
    #[error("invalid remote frame length {0}")]
    Length(u32),
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
}

fn fill(reader: &mut impl Read, bytes: &mut [u8], clean_eof: bool) -> Result<bool, FrameError> {
    let mut offset = 0;
    while offset < bytes.len() {
        match reader.read(&mut bytes[offset..]) {
            Ok(0) if offset == 0 && clean_eof => return Ok(false),
            Ok(0) => return Err(FrameError::Truncated),
            Ok(count) => offset += count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error.into()),
        }
    }
    Ok(true)
}

/// `None` is a clean EOF before the next prefix, not a partial-frame success.
pub fn read_frame(reader: &mut impl Read) -> Result<Option<Envelope>, FrameError> {
    let mut prefix = [0; 4];
    if !fill(reader, &mut prefix, true)? {
        return Ok(None);
    }
    let length = u32::from_be_bytes(prefix);
    if length == 0 || length as usize > MAX_FRAME_BYTES {
        return Err(FrameError::Length(length));
    }
    let mut body = vec![0; length as usize];
    fill(reader, &mut body, false)?;
    Ok(Some(Envelope::decode(&body)?))
}

struct BoundedBuffer {
    bytes: Vec<u8>,
    limit: usize,
    exceeded: bool,
}

impl Write for BoundedBuffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.limit - self.bytes.len() {
            self.exceeded = true;
            return Err(io::Error::other("remote channel byte limit"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Validate and bound the complete message before emitting any bytes.
pub fn write_frame(writer: &mut impl Write, envelope: &Envelope) -> Result<(), FrameError> {
    envelope.validate()?;
    let mut buffer = BoundedBuffer {
        bytes: Vec::new(),
        limit: envelope.channel.max_bytes(),
        exceeded: false,
    };
    if let Err(error) = serde_json::to_writer(&mut buffer, envelope) {
        return Err(if buffer.exceeded {
            ProtocolError::Limit("channel bytes").into()
        } else {
            ProtocolError::Json(error).into()
        });
    }
    writer.write_all(&(buffer.bytes.len() as u32).to_be_bytes())?;
    writer.write_all(&buffer.bytes)?;
    writer.flush()?;
    Ok(())
}
