//! A real interactive shell bound to a working directory (ZS1-166).
//!
//! The Shell pane is not a projection: it owns a pseudo-terminal running the
//! user's login shell with the pane's working directory, forwards keystrokes,
//! and paints a small terminal screen parsed from the PTY byte stream.
//!
//! The screen model is deliberately minimal but *correct* for interactive line
//! editing — notably, `\b` moves the cursor left (it does not erase), CSI
//! cursor movement and erase-to-end-of-line are honored, and East-Asian wide
//! characters occupy two cells. A naive "strip escapes" renderer corrupts CJK
//! editing; this one tracks a cell grid and the live cursor (ZS1-173).
//!
//! On non-unix targets the type exists but [`PtyShell::spawn`] always fails so
//! callers fall back to the last snapshot.

use std::path::PathBuf;

#[cfg(unix)]
pub use imp::{PtyShell, key_bytes};

#[cfg(not(unix))]
pub use fallback::{PtyShell, key_bytes};

#[cfg(unix)]
mod imp {
    use super::PathBuf;
    use std::{
        ffi::CString,
        fmt, io,
        os::{fd::RawFd, unix::ffi::OsStrExt},
        sync::{
            Arc, Mutex,
            atomic::{AtomicBool, Ordering},
            mpsc::{self, Receiver, SyncSender, TrySendError},
        },
        thread::JoinHandle,
    };
    use unicode_width::UnicodeWidthChar;

    /// Upper bound on retained screen lines (scrollback) and columns.
    const MAX_LINES: usize = 4_000;
    const MAX_COLS: usize = 512;
    const OUTPUT_CHUNK_BYTES: usize = 8192;
    const OUTPUT_QUEUE_CHUNKS: usize = 32;
    const PUMP_CHUNKS: usize = 8;
    const MAX_ESCAPE_BYTES: usize = 4096;

    enum DiscardEscape {
        Csi,
        Osc { escape: bool },
    }

    fn queue_output(tx: &SyncSender<Vec<u8>>, mut chunk: Vec<u8>, stop: &AtomicBool) -> bool {
        loop {
            if stop.load(Ordering::Acquire) {
                return false;
            }
            match tx.try_send(chunk) {
                Ok(()) => return true,
                Err(TrySendError::Disconnected(_)) => return false,
                Err(TrySendError::Full(bytes)) => {
                    chunk = bytes;
                    std::thread::sleep(std::time::Duration::from_millis(2));
                }
            }
        }
    }

    /// One screen cell. `width == 0` marks the second cell of a wide glyph.
    #[derive(Clone, Copy)]
    pub struct Cell {
        ch: char,
        width: u8,
    }

    impl Cell {
        pub const BLANK: Self = Self { ch: ' ', width: 1 };
    }

    /// A bounded character grid produced by a minimal VT parser.
    pub struct Screen {
        pub lines: Vec<Vec<Cell>>,
        soft_wrapped: Vec<bool>,
        discard_escape: Option<DiscardEscape>,
        pub row: usize,
        pub col: usize,
    }

    impl Screen {
        pub fn new() -> Self {
            Self {
                lines: vec![Vec::new()],
                soft_wrapped: vec![false],
                discard_escape: None,
                row: 0,
                col: 0,
            }
        }

        fn ensure_row(&mut self) {
            while self.lines.len() <= self.row {
                self.lines.push(Vec::new());
                self.soft_wrapped.push(false);
            }
        }

        fn cell_mut(&mut self, row: usize, col: usize) -> &mut Cell {
            self.ensure_row();
            let line = &mut self.lines[row];
            while line.len() <= col {
                line.push(Cell::BLANK);
            }
            &mut line[col]
        }

        fn advance_line(&mut self, wrapped: bool) {
            self.row += 1;
            self.col = 0;
            self.ensure_row();
            self.soft_wrapped[self.row] = wrapped;
            while self.lines.len() > MAX_LINES {
                self.lines.remove(0);
                self.soft_wrapped.remove(0);
                self.row = self.row.saturating_sub(1);
            }
        }

        fn put(&mut self, ch: char) {
            let width = UnicodeWidthChar::width(ch).unwrap_or(0) as u8;
            if width == 0 {
                // Combining mark / zero-width: attach is out of scope; drop it
                // rather than corrupt the grid.
                return;
            }
            let cells = usize::from(width.min(2));
            if self.col + cells > MAX_COLS {
                self.advance_line(true);
            }
            self.cell_mut(self.row, self.col).ch = ch;
            self.cell_mut(self.row, self.col).width = width;
            if cells == 2 {
                let continuation = self.cell_mut(self.row, self.col + 1);
                continuation.ch = ' ';
                continuation.width = 0;
            }
            self.col += cells;
        }

        fn carriage_return(&mut self) {
            self.col = 0;
        }

        /// Backspace moves the cursor exactly one cell left; it never erases.
        /// Wide glyphs are crossed by the shell emitting two backspaces, so
        /// this must not skip a continuation cell on its own (ZS1-173).
        fn backspace(&mut self) {
            self.col = self.col.saturating_sub(1);
        }

        fn tab(&mut self) {
            let next = (self.col / 8 + 1) * 8;
            self.col = next.min(MAX_COLS.saturating_sub(1));
        }

        fn erase_line(&mut self, mode: usize) {
            self.ensure_row();
            let col = self.col;
            let line = &mut self.lines[self.row];
            match mode {
                0 => {
                    if col < line.len() {
                        line.truncate(col);
                    }
                }
                1 => {
                    for cell in line.iter_mut().take(col.saturating_add(1)) {
                        *cell = Cell::BLANK;
                    }
                }
                _ => line.clear(),
            }
        }

        fn erase_display(&mut self, mode: usize) {
            match mode {
                0 => {
                    self.erase_line(0);
                    self.lines.truncate(self.row + 1);
                    self.soft_wrapped.truncate(self.row + 1);
                }
                1 => {
                    for row in 0..self.row {
                        self.lines[row].clear();
                    }
                    self.erase_line(1);
                }
                _ => {
                    self.lines.clear();
                    self.lines.push(Vec::new());
                    self.soft_wrapped.clear();
                    self.soft_wrapped.push(false);
                    self.row = 0;
                    self.col = 0;
                }
            }
        }

        fn move_to(&mut self, row: usize, col: usize) {
            self.row = row.min(MAX_LINES.saturating_sub(1));
            self.col = col.min(MAX_COLS.saturating_sub(1));
            self.ensure_row();
        }

        fn csi(&mut self, params: &[u8], final_byte: u8) {
            if params.first() == Some(&b'?') || params.first() == Some(&b'>') {
                return;
            }
            let numbers: Vec<usize> = params
                .split(|byte| *byte == b';')
                .map(|part| {
                    std::str::from_utf8(part)
                        .ok()
                        .and_then(|text| text.parse().ok())
                        .unwrap_or(0)
                })
                .collect();
            let arg = |index: usize| -> usize {
                let value = numbers.get(index).copied().unwrap_or(0);
                if value == 0 { 1 } else { value }
            };
            match final_byte {
                b'A' => self.row = self.row.saturating_sub(arg(0)),
                b'B' => self.row = (self.row + arg(0)).min(MAX_LINES.saturating_sub(1)),
                b'C' => self.col = (self.col + arg(0)).min(MAX_COLS.saturating_sub(1)),
                b'D' => self.col = self.col.saturating_sub(arg(0)),
                b'E' => {
                    self.row = (self.row + arg(0)).min(MAX_LINES.saturating_sub(1));
                    self.col = 0;
                }
                b'F' => {
                    self.row = self.row.saturating_sub(arg(0));
                    self.col = 0;
                }
                b'G' | b'`' => self.col = (arg(0) - 1).min(MAX_COLS.saturating_sub(1)),
                b'd' => self.row = (arg(0) - 1).min(MAX_LINES.saturating_sub(1)),
                b'H' | b'f' => {
                    let row = numbers.first().copied().unwrap_or(0);
                    let col = numbers.get(1).copied().unwrap_or(0);
                    self.move_to(row.saturating_sub(1), col.saturating_sub(1));
                }
                b'J' => self.erase_display(numbers.first().copied().unwrap_or(0)),
                b'K' => self.erase_line(numbers.first().copied().unwrap_or(0)),
                _ => {}
            }
            self.ensure_row();
        }

        fn line_text(&self, row: usize) -> String {
            let mut text = String::new();
            if let Some(line) = self.lines.get(row) {
                for cell in line {
                    if cell.width == 0 {
                        continue;
                    }
                    text.push(cell.ch);
                }
            }
            text.trim_end().to_owned()
        }

        pub fn content(&self, rows: usize) -> String {
            let total = self.lines.len();
            let start = total.saturating_sub(rows.max(1));
            (start..total)
                .map(|row| self.line_text(row))
                .collect::<Vec<_>>()
                .join("\n")
        }

        pub fn remote_content(&self, rows: usize) -> String {
            let total = self.lines.len();
            let mut start = total.saturating_sub(rows.max(1));
            // Include a retained logical line's prefix before redaction, even
            // when the requested display window starts inside a soft wrap.
            while start > 0 && self.soft_wrapped[start] {
                start -= 1;
            }
            let mut text = String::new();
            let mut lost_prefix = self.soft_wrapped[start];
            for row in start..total {
                if row > start && !self.soft_wrapped[row] {
                    text.push('\n');
                    lost_prefix = false;
                }
                if lost_prefix {
                    if row == start {
                        text.push_str("<terminal line prefix unavailable>");
                    }
                } else {
                    text.push_str(&self.line_text(row));
                }
            }
            text
        }
    }

    fn utf8_len(byte: u8) -> usize {
        match byte {
            0x00..=0x7f => 1,
            0xc0..=0xdf => 2,
            0xe0..=0xef => 3,
            0xf0..=0xf7 => 4,
            _ => 1,
        }
    }

    /// Parse one escape sequence starting at `bytes[0] == ESC`. Returns the
    /// number of bytes consumed, or `None` when the sequence is incomplete.
    fn parse_escape(screen: &mut Screen, bytes: &[u8]) -> Option<usize> {
        let next = *bytes.get(1)?;
        match next {
            b'[' => {
                let mut index = 2;
                while index < bytes.len() && !(0x40..=0x7e).contains(&bytes[index]) {
                    index += 1;
                }
                if index >= bytes.len() {
                    return None;
                }
                screen.csi(&bytes[2..index], bytes[index]);
                Some(index + 1)
            }
            b']' => {
                let mut index = 2;
                while index < bytes.len() {
                    if bytes[index] == 0x07 {
                        return Some(index + 1);
                    }
                    if bytes[index] == 0x1b && bytes.get(index + 1) == Some(&b'\\') {
                        return Some(index + 2);
                    }
                    index += 1;
                }
                None
            }
            _ => Some(2),
        }
    }

    /// Consume every complete sequence from `pending`, leaving a trailing
    /// partial UTF-8 or escape sequence for the next chunk.
    pub fn feed(screen: &mut Screen, pending: &mut Vec<u8>) {
        let mut index = 0;
        while index < pending.len() {
            let byte = pending[index];
            if let Some(discard) = &mut screen.discard_escape {
                let finished = match discard {
                    DiscardEscape::Csi => (0x40..=0x7e).contains(&byte),
                    DiscardEscape::Osc { escape } => {
                        let finished = byte == 0x07 || *escape && byte == b'\\';
                        *escape = byte == 0x1b;
                        finished
                    }
                };
                if finished {
                    screen.discard_escape = None;
                }
                index += 1;
                continue;
            }
            match byte {
                0x1b => match parse_escape(screen, &pending[index..]) {
                    Some(used) => index += used,
                    None if pending.len() - index > MAX_ESCAPE_BYTES => {
                        screen.discard_escape = Some(if pending.get(index + 1) == Some(&b']') {
                            DiscardEscape::Osc { escape: false }
                        } else {
                            DiscardEscape::Csi
                        });
                        index += 2;
                    }
                    None => break,
                },
                b'\n' => {
                    screen.advance_line(false);
                    index += 1;
                }
                b'\r' => {
                    screen.carriage_return();
                    index += 1;
                }
                0x08 => {
                    screen.backspace();
                    index += 1;
                }
                b'\t' => {
                    screen.tab();
                    index += 1;
                }
                0x00..=0x1f | 0x7f => index += 1,
                _ => {
                    let width = utf8_len(byte);
                    if index + width > pending.len() {
                        break;
                    }
                    match std::str::from_utf8(&pending[index..index + width]) {
                        Ok(text) => {
                            for character in text.chars() {
                                screen.put(character);
                            }
                            index += width;
                        }
                        Err(_) => index += 1,
                    }
                }
            }
        }
        pending.drain(..index);
    }

    pub struct PtyShell {
        inner: Arc<Mutex<Inner>>,
    }

    struct Inner {
        instance_id: String,
        local_size: Option<(u16, u16)>,
        master: RawFd,
        child: libc::pid_t,
        exited: bool,
        lifecycle_dirty: bool,
        cwd: PathBuf,
        screen: Screen,
        pending: Vec<u8>,
        /// Bytes accepted before the slave could read (early keystrokes, or a
        /// full kernel buffer). Flushed opportunistically, never blocking.
        input: Vec<u8>,
        rx: Receiver<Vec<u8>>,
        reader_stop: Arc<AtomicBool>,
        reader: Option<JoinHandle<()>>,
    }

    fn running(inner: &mut Inner) -> bool {
        if inner.exited {
            return false;
        }
        let mut status = 0;
        // Reap only this owned child, and never signal its PID after reaping.
        let result = unsafe { libc::waitpid(inner.child, &mut status, libc::WNOHANG) };
        if result == inner.child
            || result < 0 && io::Error::last_os_error().raw_os_error() == Some(libc::ECHILD)
        {
            inner.exited = true;
            inner.lifecycle_dirty = true;
            inner.input.clear();
        }
        !inner.exited
    }

    /// Best-effort flush of buffered input. EAGAIN keeps the remainder queued.
    fn flush_input(inner: &mut Inner) -> io::Result<()> {
        while !inner.input.is_empty() {
            // SAFETY: `master` is an open fd for the lifetime of `Inner`.
            let written = unsafe {
                libc::write(
                    inner.master,
                    inner.input.as_ptr() as *const libc::c_void,
                    inner.input.len(),
                )
            };
            if written > 0 {
                inner.input.drain(..written as usize);
            } else {
                let error = io::Error::last_os_error();
                match error.kind() {
                    io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock => break,
                    _ => {
                        inner.input.clear();
                        return Err(error);
                    }
                }
            }
        }
        Ok(())
    }

    impl Clone for PtyShell {
        fn clone(&self) -> Self {
            Self {
                inner: Arc::clone(&self.inner),
            }
        }
    }

    impl fmt::Debug for PtyShell {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self.inner.lock() {
                Ok(inner) => formatter
                    .debug_struct("PtyShell")
                    .field("pid", &inner.child)
                    .field("cwd", &inner.cwd)
                    .finish(),
                Err(_) => formatter
                    .debug_struct("PtyShell")
                    .field("pid", &"<poisoned>")
                    .finish(),
            }
        }
    }

    impl PtyShell {
        /// Spawn `$SHELL` (or `/bin/sh`) in a fresh PTY rooted at `cwd`.
        pub fn spawn(cwd: PathBuf) -> io::Result<Self> {
            let mut nonce = [0u8; 32];
            getrandom::getrandom(&mut nonce)
                .map_err(|error| io::Error::other(error.to_string()))?;
            use sha2::{Digest, Sha256};
            let instance_id = format!("{:x}", Sha256::digest(nonce));
            let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_owned());
            let shell_c = CString::new(shell.clone())
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid $SHELL"))?;
            let cwd_c = CString::new(cwd.as_os_str().as_bytes())
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid cwd"))?;
            let term_c = CString::new("TERM=xterm-256color").expect("static TERM");

            // SAFETY: `posix_openpt` returns an owned fd or -1.
            let master = unsafe { libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY) };
            if master < 0 {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: `master` is an open PTY master.
            if unsafe { libc::grantpt(master) } != 0 || unsafe { libc::unlockpt(master) } != 0 {
                let error = io::Error::last_os_error();
                unsafe { libc::close(master) };
                return Err(error);
            }
            // SAFETY: `master` is unlocked; `ptsname` returns a libc-owned buffer
            // that stays valid until the next `ptsname` call.
            let slave_name = unsafe { libc::ptsname(master) };
            if slave_name.is_null() {
                let error = io::Error::last_os_error();
                unsafe { libc::close(master) };
                return Err(error);
            }

            // Fork before allocating further state so the child only uses
            // async-signal-safe calls before `execvp`.
            // SAFETY: called in a normal multi-threaded process, not mid-signal.
            let pid = unsafe { libc::fork() };
            if pid < 0 {
                let error = io::Error::last_os_error();
                unsafe { libc::close(master) };
                return Err(error);
            }
            if pid == 0 {
                // Child: attach the controlling terminal and replace the image.
                unsafe {
                    libc::setsid();
                    let slave = libc::open(slave_name, libc::O_RDWR);
                    if slave < 0 {
                        libc::_exit(127);
                    }
                    libc::ioctl(slave, libc::TIOCSCTTY as libc::c_ulong, 0);
                    libc::dup2(slave, 0);
                    libc::dup2(slave, 1);
                    libc::dup2(slave, 2);
                    if slave > 2 {
                        libc::close(slave);
                    }
                    libc::close(master);
                    libc::chdir(cwd_c.as_ptr());
                    libc::putenv(term_c.into_raw());
                    let argument = shell_c.as_ptr() as *const libc::c_char;
                    let argv = [argument, std::ptr::null()];
                    libc::execvp(shell_c.as_ptr(), argv.as_ptr());
                    libc::_exit(127);
                }
            }

            // SAFETY: setting O_NONBLOCK on a master fd this process owns.
            unsafe {
                let flags = libc::fcntl(master, libc::F_GETFL);
                if flags >= 0 {
                    libc::fcntl(master, libc::F_SETFL, flags | libc::O_NONBLOCK);
                }
            }

            let (tx, rx) = mpsc::sync_channel(OUTPUT_QUEUE_CHUNKS);
            let reader_stop = Arc::new(AtomicBool::new(false));
            let stop = Arc::clone(&reader_stop);
            let reader = std::thread::spawn(move || {
                let mut buffer = [0u8; OUTPUT_CHUNK_BYTES];
                loop {
                    if stop.load(Ordering::Acquire) {
                        break;
                    }
                    // SAFETY: `master` stays open until this thread is joined in
                    // `Inner::drop`, so it is never reused underneath us.
                    let read = unsafe {
                        libc::read(
                            master,
                            buffer.as_mut_ptr() as *mut libc::c_void,
                            buffer.len(),
                        )
                    };
                    if read > 0 {
                        if !queue_output(&tx, buffer[..read as usize].to_vec(), &stop) {
                            break;
                        }
                    } else if read == 0 {
                        break;
                    } else {
                        let error = io::Error::last_os_error();
                        match error.kind() {
                            io::ErrorKind::Interrupted => continue,
                            io::ErrorKind::WouldBlock => {
                                std::thread::sleep(std::time::Duration::from_millis(2));
                            }
                            _ => break,
                        }
                    }
                }
            });

            Ok(Self {
                inner: Arc::new(Mutex::new(Inner {
                    instance_id,
                    local_size: None,
                    master,
                    child: pid,
                    exited: false,
                    lifecycle_dirty: false,
                    cwd,
                    screen: Screen::new(),
                    pending: Vec::new(),
                    input: Vec::new(),
                    rx,
                    reader_stop,
                    reader: Some(reader),
                })),
            })
        }

        pub fn cwd(&self) -> PathBuf {
            self.inner
                .lock()
                .map(|inner| inner.cwd.clone())
                .unwrap_or_default()
        }

        /// Drain pending output into the screen. Returns whether anything
        /// changed so the caller can mark the frame dirty.
        pub fn pump(&mut self) -> bool {
            let Ok(mut inner) = self.inner.lock() else {
                return false;
            };
            let mut changed = false;
            for _ in 0..PUMP_CHUNKS {
                let Ok(chunk) = inner.rx.try_recv() else {
                    break;
                };
                // Split the borrow so the parser can touch both fields.
                let Inner {
                    screen, pending, ..
                } = &mut *inner;
                pending.extend_from_slice(&chunk);
                feed(screen, pending);
                changed = true;
            }
            if running(&mut inner) {
                let _ = flush_input(&mut inner);
            }
            changed |= std::mem::take(&mut inner.lifecycle_dirty);
            changed
        }

        pub fn write_input(&mut self, bytes: &[u8]) {
            if bytes.is_empty() {
                return;
            }
            let Ok(mut inner) = self.inner.lock() else {
                return;
            };
            if !running(&mut inner) {
                return;
            }
            inner.input.extend_from_slice(bytes);
            let _ = flush_input(&mut inner);
        }

        pub fn instance_id(&self) -> String {
            self.inner
                .lock()
                .map(|inner| inner.instance_id.clone())
                .unwrap_or_default()
        }

        pub fn is_running(&self) -> bool {
            self.inner
                .lock()
                .map(|mut inner| running(&mut inner))
                .unwrap_or(false)
        }

        pub fn size(&self) -> io::Result<(u16, u16)> {
            let inner = self
                .inner
                .lock()
                .map_err(|_| io::Error::other("PTY lock unavailable"))?;
            let mut size = libc::winsize {
                ws_row: 0,
                ws_col: 0,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };
            // SAFETY: the owned master fd and output winsize stay valid under the lock.
            if unsafe { libc::ioctl(inner.master, libc::TIOCGWINSZ as libc::c_ulong, &mut size) }
                < 0
            {
                return Err(io::Error::last_os_error());
            }
            Ok((size.ws_row, size.ws_col))
        }

        /// Remote admission is bounded even when the slave is not reading.
        pub fn try_write_input(&mut self, bytes: &[u8]) -> io::Result<()> {
            let mut inner = self
                .inner
                .lock()
                .map_err(|_| io::Error::other("PTY lock unavailable"))?;
            if !running(&mut inner) {
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "PTY shell exited",
                ));
            }
            if bytes.is_empty()
                || bytes.len() > 4096
                || inner.input.len().saturating_add(bytes.len()) > 16 * 1024
            {
                return Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "PTY input queue full",
                ));
            }
            inner.input.extend_from_slice(bytes);
            flush_input(&mut inner)
        }

        pub fn try_resize(&mut self, rows: u16, cols: u16) -> io::Result<()> {
            if rows == 0 || cols == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "invalid PTY size",
                ));
            }
            let mut inner = self
                .inner
                .lock()
                .map_err(|_| io::Error::other("PTY lock unavailable"))?;
            if !running(&mut inner) {
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "PTY shell exited",
                ));
            }
            let size = libc::winsize {
                ws_row: rows,
                ws_col: cols,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };
            // SAFETY: the owned master fd and winsize remain valid under the lock.
            if unsafe { libc::ioctl(inner.master, libc::TIOCSWINSZ as libc::c_ulong, &size) } < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }

        pub fn resize(&mut self, rows: u16, cols: u16) {
            if rows == 0 || cols == 0 {
                return;
            }
            let Ok(mut inner) = self.inner.lock() else {
                return;
            };
            // A remote size remains effective until desktop geometry changes,
            // rather than being immediately overwritten by the next redraw.
            if inner.local_size == Some((rows, cols)) {
                return;
            }
            inner.local_size = Some((rows, cols));
            let size = libc::winsize {
                ws_row: rows,
                ws_col: cols,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };
            // SAFETY: `master` is an open PTY master; `size` is a valid winsize.
            unsafe {
                libc::ioctl(inner.master, libc::TIOCSWINSZ as libc::c_ulong, &size);
            }
        }

        /// Bounded last `rows` lines of the screen.
        pub fn content(&self, rows: usize) -> String {
            let Ok(inner) = self.inner.lock() else {
                return String::new();
            };
            inner.screen.content(rows)
        }

        pub fn remote_content(&self, rows: usize) -> String {
            let Ok(inner) = self.inner.lock() else {
                return String::new();
            };
            inner.screen.remote_content(rows)
        }

        /// Live cursor as `(column, row)` in display cells.
        pub fn cursor(&self) -> (usize, usize) {
            let Ok(inner) = self.inner.lock() else {
                return (0, 0);
            };
            (inner.screen.col, inner.screen.row)
        }

        /// Number of screen lines (scrollback included).
        pub fn line_count(&self) -> usize {
            let Ok(inner) = self.inner.lock() else {
                return 1;
            };
            inner.screen.lines.len().max(1)
        }
    }

    impl Drop for Inner {
        fn drop(&mut self) {
            // A slave descendant may keep the master readable/EAGAIN after the
            // shell exits. Stop the nonblocking reader before joining it.
            self.reader_stop.store(true, Ordering::Release);
            // SAFETY: `child` is this shell; the master stays open until join.
            unsafe {
                if running(self) {
                    libc::kill(self.child, libc::SIGHUP);
                    libc::kill(self.child, libc::SIGKILL);
                    let mut status = 0;
                    libc::waitpid(self.child, &mut status, 0);
                }
            }
            if let Some(reader) = self.reader.take() {
                let _ = reader.join();
            }
            // SAFETY: closing a master fd owned exactly once.
            unsafe {
                libc::close(self.master);
            }
        }
    }

    /// Map a crossterm key to the bytes a terminal would send. Returns `None`
    /// for chords the TUI keeps for itself.
    pub fn key_bytes(
        code: &crossterm::event::KeyCode,
        modifiers: crossterm::event::KeyModifiers,
    ) -> Option<Vec<u8>> {
        use crossterm::event::{KeyCode, KeyModifiers};
        let ctrl = modifiers.contains(KeyModifiers::CONTROL);
        if modifiers.contains(KeyModifiers::ALT) {
            return None;
        }
        let bytes = match code {
            KeyCode::Char(c) => {
                if ctrl {
                    // Only forward control chords a shell actually uses; the
                    // rest belong to the TUI (tabs, close pane, editor, ...).
                    let lower = c.to_ascii_lowercase();
                    match lower {
                        'c' | 'd' | 'z' | 'a' | 'e' | 'l' => vec![(lower as u8) - b'a' + 1],
                        _ => return None,
                    }
                } else {
                    c.to_string().into_bytes()
                }
            }
            KeyCode::Enter => vec![b'\r'],
            KeyCode::Backspace => vec![0x7f],
            KeyCode::Delete => b"\x1b[3~".to_vec(),
            KeyCode::Esc => vec![0x1b],
            KeyCode::Up => b"\x1b[A".to_vec(),
            KeyCode::Down => b"\x1b[B".to_vec(),
            KeyCode::Right => b"\x1b[C".to_vec(),
            KeyCode::Left => b"\x1b[D".to_vec(),
            KeyCode::Home => b"\x1b[H".to_vec(),
            KeyCode::End => b"\x1b[F".to_vec(),
            KeyCode::PageUp => b"\x1b[5~".to_vec(),
            KeyCode::PageDown => b"\x1b[6~".to_vec(),
            KeyCode::Insert => b"\x1b[2~".to_vec(),
            KeyCode::Tab
            | KeyCode::BackTab
            | KeyCode::F(_)
            | KeyCode::Null
            | KeyCode::CapsLock
            | KeyCode::ScrollLock
            | KeyCode::NumLock
            | KeyCode::PrintScreen
            | KeyCode::Pause
            | KeyCode::Menu
            | KeyCode::KeypadBegin
            | KeyCode::Media(_)
            | KeyCode::Modifier(_) => return None,
        };
        Some(bytes)
    }

    #[cfg(test)]
    mod output_bounds_tests {
        use super::*;

        #[test]
        fn queue_backpressure_preserves_bytes_and_stop_cancels_full_queue() {
            let (tx, rx) = mpsc::sync_channel(OUTPUT_QUEUE_CHUNKS);
            let stop = Arc::new(AtomicBool::new(false));
            for index in 0..OUTPUT_QUEUE_CHUNKS {
                assert!(queue_output(
                    &tx,
                    vec![index as u8; OUTPUT_CHUNK_BYTES],
                    &stop
                ));
            }
            assert!(matches!(tx.try_send(vec![255]), Err(TrySendError::Full(_))));
            let (ready_tx, ready_rx) = mpsc::channel();
            let worker_stop = Arc::clone(&stop);
            let worker = std::thread::spawn(move || {
                ready_tx.send(()).unwrap();
                queue_output(&tx, vec![255; OUTPUT_CHUNK_BYTES], &worker_stop)
            });
            ready_rx.recv().unwrap();
            assert_eq!(rx.recv().unwrap(), vec![0; OUTPUT_CHUNK_BYTES]);
            assert!(worker.join().unwrap());
            for index in 1..OUTPUT_QUEUE_CHUNKS {
                assert_eq!(rx.recv().unwrap(), vec![index as u8; OUTPUT_CHUNK_BYTES]);
            }
            assert_eq!(rx.recv().unwrap(), vec![255; OUTPUT_CHUNK_BYTES]);

            let (tx, _rx) = mpsc::sync_channel(1);
            tx.send(vec![1]).unwrap();
            let worker_stop = Arc::clone(&stop);
            let (done_tx, done_rx) = mpsc::channel();
            let worker = std::thread::spawn(move || {
                done_tx
                    .send(queue_output(&tx, vec![2], &worker_stop))
                    .unwrap();
            });
            stop.store(true, Ordering::Release);
            assert!(
                !done_rx
                    .recv_timeout(std::time::Duration::from_secs(1))
                    .unwrap()
            );
            worker.join().unwrap();
        }

        #[test]
        fn oversized_incomplete_escape_is_bounded_and_not_reinterpreted_as_text() {
            for prefix in [b"\x1b]".as_slice(), b"\x1b[".as_slice()] {
                let mut screen = Screen::new();
                let mut pending = prefix.to_vec();
                pending.extend(vec![b'1'; MAX_ESCAPE_BYTES + 1]);
                feed(&mut screen, &mut pending);
                assert!(pending.is_empty());
                assert_eq!(screen.content(64), "");
                for _ in 0..20 {
                    pending.extend(vec![b'2'; OUTPUT_CHUNK_BYTES]);
                    feed(&mut screen, &mut pending);
                    assert!(pending.is_empty());
                }
                pending.extend(if prefix[1] == b']' {
                    b"\x1b".as_slice()
                } else {
                    b"m".as_slice()
                });
                feed(&mut screen, &mut pending);
                pending.extend(if prefix[1] == b']' {
                    b"\\visible".as_slice()
                } else {
                    b"visible".as_slice()
                });
                feed(&mut screen, &mut pending);
                assert_eq!(screen.content(64), "visible");
                assert!(pending.is_empty());
            }
        }

        #[test]
        fn one_pump_consumes_only_its_budget_and_keeps_the_remaining_fifo() {
            let root = tempfile::tempdir().unwrap();
            let mut shell = PtyShell::spawn(root.path().to_path_buf()).unwrap();
            let (tx, rx) = mpsc::sync_channel(OUTPUT_QUEUE_CHUNKS);
            shell.inner.lock().unwrap().rx = rx;
            for index in 0..OUTPUT_QUEUE_CHUNKS {
                tx.send(format!("{index}\n").into_bytes()).unwrap();
            }
            assert!(shell.pump());
            let inner = shell.inner.lock().unwrap();
            assert_eq!(inner.screen.lines.len(), PUMP_CHUNKS + 1);
            for index in PUMP_CHUNKS..OUTPUT_QUEUE_CHUNKS {
                assert_eq!(
                    inner.rx.try_recv().unwrap(),
                    format!("{index}\n").into_bytes()
                );
            }
            assert!(inner.rx.try_recv().is_err());
        }
    }
}

#[cfg(not(unix))]
mod fallback {
    use std::io;
    use std::path::PathBuf;

    #[derive(Clone, Debug)]
    pub struct PtyShell;

    impl PtyShell {
        pub fn is_running(&self) -> bool {
            false
        }
        pub fn instance_id(&self) -> String {
            String::new()
        }
        pub fn size(&self) -> io::Result<(u16, u16)> {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "PTY unavailable",
            ))
        }
        pub fn try_write_input(&mut self, _bytes: &[u8]) -> io::Result<()> {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "PTY unavailable",
            ))
        }
        pub fn try_resize(&mut self, _rows: u16, _cols: u16) -> io::Result<()> {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "PTY unavailable",
            ))
        }
        pub fn spawn(_cwd: PathBuf) -> io::Result<Self> {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "interactive shell requires a unix pseudo-terminal",
            ))
        }

        pub fn cwd(&self) -> PathBuf {
            PathBuf::new()
        }

        pub fn pump(&mut self) -> bool {
            false
        }

        pub fn write_input(&mut self, _bytes: &[u8]) {}

        pub fn resize(&mut self, _rows: u16, _cols: u16) {}

        pub fn content(&self, _rows: usize) -> String {
            String::new()
        }

        pub fn remote_content(&self, _rows: usize) -> String {
            String::new()
        }

        pub fn cursor(&self) -> (usize, usize) {
            (0, 0)
        }

        pub fn line_count(&self) -> usize {
            1
        }
    }

    pub fn key_bytes(
        _code: &crossterm::event::KeyCode,
        _modifiers: crossterm::event::KeyModifiers,
    ) -> Option<Vec<u8>> {
        None
    }
}

/// Resolve the shell working directory from an ordered candidate list.
pub fn resolve_cwd(candidates: impl IntoIterator<Item = PathBuf>) -> PathBuf {
    for candidate in candidates {
        if candidate.is_dir() {
            return candidate;
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn render(bytes: &[u8]) -> String {
        let mut screen = imp::Screen::new();
        let mut pending = bytes.to_vec();
        imp::feed(&mut screen, &mut pending);
        screen.content(screen.lines.len())
    }

    #[cfg(unix)]
    #[test]
    fn backspace_moves_the_cursor_and_never_erases() {
        // Backspace only moves; the glyphs stay until overwritten.
        assert_eq!(render(b"abc\x08\x08"), "abc");
        // Writing after moving left overwrites in place.
        assert_eq!(render(b"ab\x08X"), "aX");
    }

    #[cfg(unix)]
    #[test]
    fn wide_characters_use_two_cells_and_cursor_tracks_them() {
        let mut screen = imp::Screen::new();
        let mut pending = "你好".to_string().into_bytes();
        pending.push(0x08);
        imp::feed(&mut screen, &mut pending);
        assert_eq!(screen.content(10), "你好");
        // Backspace moves exactly one cell (into the second glyph).
        assert_eq!(screen.col, 3);
    }

    #[cfg(unix)]
    #[test]
    fn zle_wide_erase_sequence_removes_one_glyph() {
        // zsh zle erases a wide glyph with two single-cell backspaces, two
        // spaces, then two more backspaces — exactly as observed on the wire.
        let mut bytes = "echo 你好世界".as_bytes().to_vec();
        bytes.extend_from_slice(b"\x08\x08  \x08\x08");
        assert_eq!(render(&bytes), "echo 你好世");
    }

    #[cfg(unix)]
    #[test]
    fn carriage_return_overwrites_and_csi_erase_line_clears() {
        assert_eq!(render(b"progress 10%\rprogress 100%"), "progress 100%");
        assert_eq!(render(b"stale\r\x1b[Kfresh"), "fresh");
        assert_eq!(render(b"keep\x1b[K"), "keep");
        assert_eq!(render(b"abc\x1b[2K\rxyz"), "xyz");
    }

    #[cfg(unix)]
    #[test]
    fn osc_title_is_ignored() {
        assert_eq!(render(b"\x1b]0;title\x07hello"), "hello");
    }

    #[cfg(unix)]
    #[test]
    fn remote_screen_reassembles_soft_wrap_before_secret_redaction() {
        let mut screen = imp::Screen::new();
        let mut pending =
            format!("{}api_key=WRAPPED_SECRET\nvisible", "x".repeat(509)).into_bytes();
        imp::feed(&mut screen, &mut pending);
        let local = screen.content(64);
        assert!(local.contains("api\n_key=WRAPPED_SECRET"));
        let remote = screen.remote_content(64);
        assert!(remote.contains("api_key=WRAPPED_SECRET"));
        let projected = crate::remote::terminal::screen("p", "s", "t", 0, &remote, &[]);
        assert!(
            !projected["text"]
                .as_str()
                .unwrap()
                .contains("WRAPPED_SECRET")
        );
        assert!(projected["text"].as_str().unwrap().contains("visible"));
        assert_eq!(screen.content(64), local);
        // A display window starting inside the logical line must include its prefix.
        assert!(screen.remote_content(2).contains("api_key="));
    }

    #[cfg(unix)]
    #[test]
    fn remote_screen_with_lost_wrap_prefix_withholds_until_hard_newline() {
        let mut screen = imp::Screen::new();
        let mut pending = format!("api_key={}\nvisible", "x".repeat(512 * 4001)).into_bytes();
        imp::feed(&mut screen, &mut pending);
        let remote = screen.remote_content(64);
        assert_eq!(remote, "<terminal line prefix unavailable>\nvisible");
        assert!(screen.content(64).contains('x'));
    }

    #[cfg(unix)]
    #[test]
    fn remote_wrap_metadata_survives_erase_and_cursor_rewrites() {
        let mut screen = imp::Screen::new();
        let mut pending = format!("{}wrapped", "x".repeat(512)).into_bytes();
        imp::feed(&mut screen, &mut pending);
        let mut pending = b"\x1b[2Jfresh\nnext".to_vec();
        imp::feed(&mut screen, &mut pending);
        assert_eq!(screen.remote_content(64), "fresh\nnext");
        let mut pending = b"\x1b[1;1Hreset\x1b[J".to_vec();
        imp::feed(&mut screen, &mut pending);
        assert_eq!(screen.remote_content(64), "reset");
    }

    #[test]
    fn resolve_cwd_prefers_the_first_existing_directory() {
        let dir = std::env::temp_dir();
        let missing = dir.join("zenpi-definitely-missing-dir");
        assert_eq!(resolve_cwd([missing.clone(), dir.clone()]), dir);
        assert_eq!(
            resolve_cwd([missing, PathBuf::from(".")]),
            PathBuf::from(".")
        );
    }

    #[cfg(unix)]
    #[test]
    fn spawn_runs_a_real_command_in_the_requested_cwd() {
        let dir = std::env::temp_dir();
        let mut shell = PtyShell::spawn(dir.clone()).expect("spawn shell");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while std::time::Instant::now() < deadline && shell.content(200).trim().is_empty() {
            shell.pump();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        shell.write_input(b"pwd; echo ZENPI_PTY_OK\r");
        let mut text = String::new();
        while std::time::Instant::now() < deadline {
            shell.pump();
            text = shell.content(200);
            if text.contains("ZENPI_PTY_OK") {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(text.contains("ZENPI_PTY_OK"), "no shell output: {text:?}");
        assert_eq!(shell.cwd(), dir);
    }
}
