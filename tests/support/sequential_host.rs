//! Drives the production async headless host as a client that awaits each
//! request's terminal response before writing the next frame. Test crates
//! include this file with `#[path]` so only its users compile it.

pub fn run_headless_sequential(
    agent: zenpi::core::Agent,
    input: impl AsRef<[u8]>,
    output: &mut Vec<u8>,
) -> Result<(), zenpi::headless::HeadlessError> {
    use std::{
        io::{Read, Write},
        sync::{Arc, Condvar, Mutex, mpsc},
        time::{Duration, Instant},
    };
    type Wire = Arc<(Mutex<(Vec<u8>, bool)>, Condvar)>;
    struct Writer(Wire);
    impl Write for Writer {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.0.lock().unwrap().0.extend_from_slice(bytes);
            self.0.1.notify_all();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    struct Frames(mpsc::Receiver<Vec<u8>>, Vec<u8>, usize);
    impl Read for Frames {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.2 >= self.1.len() {
                match self.0.recv() {
                    Ok(frame) => (self.1, self.2) = (frame, 0),
                    Err(_) => return Ok(0),
                }
            }
            let len = buf.len().min(self.1.len() - self.2);
            buf[..len].copy_from_slice(&self.1[self.2..self.2 + len]);
            self.2 += len;
            Ok(len)
        }
    }
    fn responses(bytes: &[u8], id: &serde_json::Value) -> usize {
        bytes
            .split(|byte| *byte == b'\n')
            .filter_map(|line| serde_json::from_slice::<serde_json::Value>(line).ok())
            .filter(|record| record["type"] == "response" && &record["id"] == id)
            .count()
    }

    let wire: Wire = Arc::new((Mutex::new((Vec::new(), false)), Condvar::new()));
    let (frames, reader) = mpsc::channel::<Vec<u8>>();
    let host_wire = Arc::clone(&wire);
    let host = std::thread::spawn(move || {
        let result = zenpi::headless::run_async_streams(
            agent,
            Frames(reader, Vec::new(), 0),
            Writer(Arc::clone(&host_wire)),
        );
        host_wire.0.lock().unwrap().1 = true;
        host_wire.1.notify_all();
        result
    });
    for frame in input.as_ref().split_inclusive(|byte| *byte == b'\n') {
        let text = std::str::from_utf8(frame).ok();
        if text.is_some_and(|text| text.trim().is_empty()) {
            let _ = frames.send(frame.to_vec());
            continue;
        }
        // The host answers unparsable frames with an uncorrelated error.
        let id = text
            .and_then(|text| zenpi::protocol::parse_line(text.trim_end_matches('\n')).ok())
            .and_then(|request| request.id().map(str::to_owned))
            .map_or(serde_json::Value::Null, serde_json::Value::String);
        let mut state = wire.0.lock().unwrap();
        if state.1 {
            break;
        }
        let before = responses(&state.0, &id);
        let _ = frames.send(frame.to_vec());
        let deadline = Instant::now() + Duration::from_secs(60);
        while !state.1 && responses(&state.0, &id) == before {
            let wait = deadline.saturating_duration_since(Instant::now());
            assert!(!wait.is_zero(), "no response to {id} within 60s");
            state = wire.1.wait_timeout(state, wait).unwrap().0;
        }
    }
    drop(frames);
    let result = host.join().expect("headless host panicked");
    output.extend_from_slice(&wire.0.lock().unwrap().0);
    result
}
