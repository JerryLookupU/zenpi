//! Search backed by an already-installed ripgrep: one `rg --json` run per
//! `search_text` call. No tool download, shell-language execution, persistent
//! index or extra service.
use crate::tools::{SupervisedChild, ToolError};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const MAX_PIPE_BYTES: usize = 256 * 1024;
const MAX_STDERR_BYTES: usize = 8192;
/// Upper bound on paths one `find` call returns.
const MAX_FIND_RESULTS: usize = 256;
const MAX_FILE_BYTES: u64 = 1024 * 1024;
const MAX_DEPTH: usize = 64;
const MAX_LINE_BYTES: usize = 2048;
const MAX_DURATION: Duration = Duration::from_secs(10);

#[derive(Debug, Clone)]
pub struct SearchBackend {
    executable: Option<PathBuf>,
}
impl SearchBackend {
    pub fn discover() -> Self {
        let executable = std::env::var_os("PATH")
            .into_iter()
            .flat_map(|paths| std::env::split_paths(&paths).collect::<Vec<_>>())
            .filter(|dir| dir.is_absolute())
            .map(|dir| dir.join("rg"))
            .find_map(|path| {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if fs::metadata(&path)
                        .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
                    {
                        return path.canonicalize().ok();
                    }
                }
                None
            });
        Self { executable }
    }
    /// Explicit host configuration, also used to test unavailable capability.
    /// Never populate this path from model tool arguments.
    pub fn configured(executable: Option<PathBuf>) -> Result<Self, ToolError> {
        if executable.as_ref().is_some_and(|path| !path.is_absolute()) {
            return Err(ToolError::InvalidArguments(
                "search executable must be host-owned absolute path".into(),
            ));
        }
        Ok(Self { executable })
    }
    pub fn executable_path(&self) -> Option<&Path> {
        self.executable.as_deref()
    }

    pub fn available(&self) -> bool {
        self.executable.is_some()
    }

    /// Search `target` with exactly one ripgrep child. Reading stops (and the
    /// child is killed) once `max_matches` matches and their trailing context
    /// are collected, or when the stdout byte cap is reached.
    pub fn search(
        &self,
        workspace: &Path,
        target: &Path,
        options: &SearchOptions,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Value, ToolError> {
        options.validate()?;
        let root = relative_root(workspace, target)?;
        let mut args = vec![
            "--json".into(),
            // Report NUL-bearing files via the end event's binary_offset
            // instead of silently skipping them during traversal.
            "--binary".into(),
            "--sort".into(),
            "path".into(),
            "--max-filesize".into(),
            MAX_FILE_BYTES.to_string(),
            "--max-count".into(),
            options.max_matches.to_string(),
            "--context".into(),
            options.context.to_string(),
        ];
        args.extend(traversal_args(
            workspace,
            options.ignore,
            options.hidden,
            MAX_DEPTH,
        ));
        if !options.regex {
            args.push("--fixed-strings".into());
        }
        if !options.case_sensitive {
            args.push("--ignore-case".into());
        }
        if let Some(glob) = &options.glob {
            args.extend(["--glob".into(), glob.clone()]);
            // A positive rg glob also whitelists hidden files (and, as in
            // pi, gitignored ones). Keep hidden=false authoritative.
            if !options.hidden {
                args.extend(["--glob".into(), "!.*".into()]);
            }
        }
        // Exclusions follow the caller glob: with rg the later glob wins.
        args.extend(exclusion_args());
        args.extend([
            "-e".into(),
            options.query.clone(),
            "--".into(),
            root.clone(),
        ]);
        let mut collector = Collector {
            workspace,
            root: PathBuf::from(&root),
            options,
            matches: Vec::new(),
            context: Vec::new(),
            reasons: BTreeSet::new(),
            admitted: BTreeMap::new(),
            last_match: None,
            files_searched: None,
        };
        let mut pending: Vec<u8> = Vec::new();
        let outcome = self.run(
            workspace,
            &args,
            Instant::now() + MAX_DURATION,
            cancel,
            &mut |chunk| {
                pending.extend_from_slice(chunk);
                let mut consumed = 0;
                let mut stop = false;
                while let Some(offset) = pending[consumed..].iter().position(|b| *b == b'\n') {
                    let line = &pending[consumed..consumed + offset];
                    consumed += offset + 1;
                    if !line.is_empty() && collector.event(line)? {
                        stop = true;
                        break;
                    }
                }
                pending.drain(..consumed);
                Ok(stop)
            },
        )?;
        match outcome.end {
            RunEnd::Truncated => {
                collector.reasons.insert("output_bytes");
                if !pending.is_empty() {
                    collector.reasons.insert("partial_event");
                }
            }
            RunEnd::Stopped => {}
            // rg exits 2 for unreadable entries after a complete walk; the
            // summary event proves the search itself ran.
            RunEnd::Exited(2) if collector.files_searched.is_some() => {
                collector.reasons.insert("search_errors");
            }
            RunEnd::Exited(0 | 1) => {}
            RunEnd::Exited(_) => return Err(rg_failed(&outcome.stderr)),
        }
        let Collector {
            matches,
            context,
            reasons,
            files_searched,
            ..
        } = collector;
        Ok(
            json!({"query":options.query,"matches":matches,"context":context,"files_searched":files_searched,
            "engine":"ripgrep","regex":options.regex,"glob":options.glob,"ignore":options.ignore,"hidden":options.hidden,
            "max_depth":MAX_DEPTH,"max_file_bytes":MAX_FILE_BYTES,
            "truncated":!reasons.is_empty(),"truncation_reasons":reasons}),
        )
    }
    // Keep the bounded search options explicit at the existing host call site.
    #[allow(clippy::too_many_arguments)]
    pub fn find(
        &self,
        workspace: &Path,
        target: &Path,
        glob: &str,
        ignore: bool,
        hidden: bool,
        limit: usize,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Value, ToolError> {
        validate_glob(glob)?;
        if !(1..=MAX_FIND_RESULTS).contains(&limit) {
            return Err(ToolError::InvalidArguments(
                "find limit must be 1..256".into(),
            ));
        }
        let mut reasons = BTreeSet::new();
        let mut files = self.files(
            workspace,
            target,
            Some(glob),
            ignore,
            hidden,
            Instant::now() + MAX_DURATION,
            cancel,
            &mut reasons,
        )?;
        if files.len() > limit {
            files.truncate(limit);
            reasons.insert("result_limit");
        }
        Ok(
            json!({"paths":files,"engine":"ripgrep","glob":glob,"ignore":ignore,"hidden":hidden,
            "max_depth":MAX_DEPTH,"truncated":!reasons.is_empty(),"truncation_reasons":reasons}),
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn files(
        &self,
        workspace: &Path,
        target: &Path,
        glob: Option<&str>,
        ignore: bool,
        hidden: bool,
        deadline: Instant,
        cancel: &dyn Fn() -> bool,
        reasons: &mut BTreeSet<&'static str>,
    ) -> Result<Vec<String>, ToolError> {
        if let Some(glob) = glob {
            validate_glob(glob)?;
        }
        let target = relative_root(workspace, target)?;
        let list = |filter: Option<&str>, ignore: bool, max_depth: usize| {
            let mut args = vec!["--files".into(), "--null".into()];
            args.extend(traversal_args(workspace, ignore, hidden, max_depth));
            if let Some(glob) = filter {
                args.extend(["--glob".into(), glob.into()]);
            }
            args.extend(exclusion_args());
            args.extend(["--".into(), target.clone()]);
            self.collect(workspace, &args, deadline, cancel)
        };
        let first = list(None, ignore, MAX_DEPTH)?;
        if first.truncated {
            reasons.insert("path_listing_bytes");
        }
        let parse = |bytes: &[u8]| -> Result<BTreeSet<String>, ToolError> {
            bytes
                .split_inclusive(|b| *b == 0)
                .filter(|piece| piece.last() == Some(&0))
                .map(|piece| {
                    let path = std::str::from_utf8(&piece[..piece.len() - 1]).map_err(|_| {
                        ToolError::InvalidArguments("non-UTF8 search result path".into())
                    })?;
                    checked_result_path(workspace, path)
                })
                .collect()
        };
        let mut candidates = parse(&first.stdout)?;
        if let Some(glob) = glob {
            // A positive rg --glob overrides .gitignore. Intersect independent
            // listings so ignore=true remains authoritative even with a glob.
            let matched = list(Some(glob), false, MAX_DEPTH)?;
            if matched.truncated {
                reasons.insert("glob_listing_bytes");
            }
            let matched = parse(&matched.stdout)?;
            candidates.retain(|path| matched.contains(path));
        }
        // ripgrep does not emit a depth-limit diagnostic. Probe with a
        // wider, still bounded depth and compare the same ignore/hidden/glob
        // projection so deep files are reported as truncated while an empty
        // directory boundary remains complete.
        if candidates.is_empty()
            && !reasons.contains("path_listing_bytes")
            && !reasons.contains("glob_listing_bytes")
        {
            let wide_first = list(None, ignore, MAX_DEPTH.saturating_mul(4))?;
            let mut wide = parse(&wide_first.stdout)?;
            if let Some(glob) = glob {
                let wide_matched = list(Some(glob), false, MAX_DEPTH.saturating_mul(4))?;
                let wide_matched = parse(&wide_matched.stdout)?;
                wide.retain(|path| wide_matched.contains(path));
            }
            if wide.iter().any(|path| !candidates.contains(path)) {
                reasons.insert("depth_limit");
            }
        }
        let mut files = Vec::new();
        for candidate in candidates {
            if cancel() {
                return Err(ToolError::Cancelled);
            }
            let path = workspace.join(&candidate);
            let metadata = fs::symlink_metadata(&path)?;
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                reasons.insert("non_regular_file");
                continue;
            }
            if crate::tool_output::private_output_path_inode(&path)? {
                continue;
            }
            let canonical = path.canonicalize()?;
            if !canonical.starts_with(workspace) || crate::tools::private_output_path(&canonical) {
                return Err(ToolError::PathDenied(candidate));
            }
            if files.len() == MAX_FIND_RESULTS {
                reasons.insert("file_limit");
                break;
            }
            files.push(candidate);
        }
        Ok(files)
    }
    /// Run one bounded ripgrep child and collect stdout (used by `find`).
    fn collect(
        &self,
        workspace: &Path,
        args: &[String],
        deadline: Instant,
        cancel: &dyn Fn() -> bool,
    ) -> Result<SearchOutput, ToolError> {
        let mut stdout = Vec::new();
        let outcome = self.run(workspace, args, deadline, cancel, &mut |chunk| {
            stdout.extend_from_slice(chunk);
            Ok(false)
        })?;
        let truncated = match outcome.end {
            RunEnd::Truncated => true,
            RunEnd::Exited(0 | 1) | RunEnd::Stopped => false,
            RunEnd::Exited(_) => return Err(rg_failed(&outcome.stderr)),
        };
        Ok(SearchOutput { stdout, truncated })
    }
    /// Run one ripgrep child, streaming at most `MAX_PIPE_BYTES` of stdout
    /// into `sink`. The sink returns `true` to stop early; the child's process
    /// group is then killed and reaped.
    #[cfg(unix)]
    fn run(
        &self,
        workspace: &Path,
        args: &[String],
        deadline: Instant,
        cancel: &dyn Fn() -> bool,
        sink: &mut dyn FnMut(&[u8]) -> Result<bool, ToolError>,
    ) -> Result<RunOutcome, ToolError> {
        use std::os::unix::process::CommandExt;
        if cancel() {
            return Err(ToolError::Cancelled);
        }
        if Instant::now() >= deadline {
            return Err(ToolError::CommandTimeout(MAX_DURATION.as_millis() as u64));
        }
        let executable = self.executable.as_ref().ok_or_else(|| {
            ToolError::Unsupported(
                "ripgrep is not installed; literal search without glob/ignore/hidden/context remains available"
                    .into(),
            )
        })?;
        let mut command = Command::new(executable);
        command.args(["--no-config", "--color", "never", "--threads", "1"]);
        command
            .args(args)
            .current_dir(workspace)
            .env_clear()
            .process_group(0)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (key, value) in crate::security::child_environment() {
            command.env(key, value);
        }
        let child = command.spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ToolError::Unsupported("configured ripgrep executable is missing".into())
            } else {
                ToolError::Io(e)
            }
        })?;
        let mut child = SupervisedChild::new(child);
        let mut stdout = child
            .child
            .stdout
            .take()
            .ok_or_else(|| ToolError::CommandFailed("search stdout missing".into()))?;
        let mut stderr = child
            .child
            .stderr
            .take()
            .ok_or_else(|| ToolError::CommandFailed("search stderr missing".into()))?;
        crate::tools::make_pipe_nonblocking(&stdout)?;
        crate::tools::make_pipe_nonblocking(&stderr)?;
        let mut read = 0_usize;
        let mut err = Vec::new();
        let mut out_eof = false;
        let mut err_eof = false;
        let end = 'supervise: loop {
            if cancel() {
                return Err(ToolError::Cancelled);
            }
            if Instant::now() >= deadline {
                return Err(ToolError::CommandTimeout(MAX_DURATION.as_millis() as u64));
            }
            let mut buffer = [0u8; 8192];
            for _ in 0..8 {
                match stdout.read(&mut buffer) {
                    Ok(0) => {
                        out_eof = true;
                        break;
                    }
                    Ok(n) => {
                        let keep = n.min(MAX_PIPE_BYTES.saturating_sub(read));
                        read += keep;
                        if keep > 0 && sink(&buffer[..keep])? {
                            break 'supervise RunEnd::Stopped;
                        }
                        if keep < n {
                            break 'supervise RunEnd::Truncated;
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e.into()),
                }
            }
            for _ in 0..8 {
                match stderr.read(&mut buffer) {
                    Ok(0) => {
                        err_eof = true;
                        break;
                    }
                    Ok(n) => {
                        let keep = n.min(MAX_STDERR_BYTES.saturating_sub(err.len()));
                        err.extend_from_slice(&buffer[..keep]);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e.into()),
                }
            }
            if let Some(status) = child.child.try_wait()?
                && out_eof
                && err_eof
            {
                // A signal-terminated child has no code; treat it as failure.
                break RunEnd::Exited(status.code().unwrap_or(-1));
            }
            // Sleep in poll(2) until output, EOF or the next cancellation check.
            use std::os::fd::AsRawFd;
            let open: Vec<_> = [(out_eof, stdout.as_raw_fd()), (err_eof, stderr.as_raw_fd())]
                .into_iter()
                .filter(|(closed, _)| !closed)
                .map(|(_, fd)| fd)
                .collect();
            let wait = if open.is_empty() {
                // Both pipes closed: the child is exiting, so reap it promptly.
                Duration::from_millis(5)
            } else {
                deadline
                    .saturating_duration_since(Instant::now())
                    .min(crate::tools::CANCEL_POLL)
            };
            crate::tools::wait_ready(&open, &[], wait);
        };
        child.cleanup();
        Ok(RunOutcome { end, stderr: err })
    }
    #[cfg(not(unix))]
    fn run(
        &self,
        _: &Path,
        _: &[String],
        _: Instant,
        _: &dyn Fn() -> bool,
        _: &mut dyn FnMut(&[u8]) -> Result<bool, ToolError>,
    ) -> Result<RunOutcome, ToolError> {
        Err(ToolError::Unsupported(
            "search process containment is unavailable on this platform".into(),
        ))
    }
}
struct SearchOutput {
    stdout: Vec<u8>,
    truncated: bool,
}
enum RunEnd {
    /// The child exited on its own with this code (-1 for a signal).
    Exited(i32),
    /// The stdout byte cap was reached; the child was killed.
    Truncated,
    /// The sink asked to stop; the child was killed.
    Stopped,
}
struct RunOutcome {
    end: RunEnd,
    stderr: Vec<u8>,
}
fn rg_failed(stderr: &[u8]) -> ToolError {
    ToolError::InvalidArguments(format!(
        "ripgrep failed: {}",
        bounded_text(&String::from_utf8_lossy(stderr), MAX_STDERR_BYTES)
    ))
}
/// Workspace-relative search root argument for ripgrep.
fn relative_root(workspace: &Path, target: &Path) -> Result<String, ToolError> {
    if !target.starts_with(workspace) || crate::tools::private_output_path(target) {
        return Err(ToolError::PathDenied(
            "search root outside workspace".into(),
        ));
    }
    let target = target
        .strip_prefix(workspace)
        .map_err(|_| ToolError::PathDenied("search root outside workspace".into()))?;
    if target.as_os_str().is_empty() {
        return Ok(".".into());
    }
    target
        .to_str()
        .map(str::to_owned)
        .ok_or_else(|| ToolError::InvalidArguments("non-UTF8 search root".into()))
}
/// Traversal flags shared by `find` listings and content search.
fn traversal_args(workspace: &Path, ignore: bool, hidden: bool, max_depth: usize) -> Vec<String> {
    let mut args = vec![
        "--max-depth".into(),
        max_depth.to_string(),
        "--no-ignore-parent".into(),
        "--no-ignore-global".into(),
        "--no-ignore-exclude".into(),
    ];
    if !workspace.join(".git").exists() {
        args.push("--no-require-git".into());
    }
    if !ignore {
        args.push("--no-ignore".into());
    }
    if hidden {
        args.push("--hidden".into());
    }
    args
}
/// Host-private output is never an ordinary search source, even with
/// hidden=true, ignore=false or a positive glob. Must follow caller globs.
fn exclusion_args() -> [String; 4] {
    [
        "--glob".into(),
        "!**/.git/**".into(),
        "--iglob".into(),
        "!**/.zenpi-output-*/**".into(),
    ]
}
/// Incremental consumer of `rg --json` events for one search.
struct Collector<'a> {
    workspace: &'a Path,
    root: PathBuf,
    options: &'a SearchOptions,
    matches: Vec<Value>,
    context: Vec<Value>,
    reasons: BTreeSet<&'static str>,
    /// Per-path admission verdicts, checked once per result file.
    admitted: BTreeMap<String, bool>,
    /// Path and line of the last admitted match once the limit is reached.
    last_match: Option<(String, u64)>,
    files_searched: Option<u64>,
}
impl Collector<'_> {
    /// Consume one complete JSON line. Returns `true` to stop the search.
    fn event(&mut self, line: &[u8]) -> Result<bool, ToolError> {
        let event: Value = serde_json::from_slice(line)
            .map_err(|_| ToolError::InvalidArguments("invalid ripgrep JSON output".into()))?;
        let kind = event["type"].as_str().unwrap_or("");
        let data = &event["data"];
        match kind {
            "summary" => {
                self.files_searched = data["stats"]["searches"].as_u64();
                return Ok(false);
            }
            "match" | "context" | "end" => {}
            _ => return Ok(false),
        }
        let binary_end = kind == "end" && data["binary_offset"].is_u64();
        if kind == "end" && !binary_end {
            // After the limit, a finished file means the trailing context is
            // complete; later files may hold more matches.
            return Ok(self.limit_reached(kind));
        }
        let Some(path) = data["path"]["text"].as_str() else {
            self.reasons.insert("non_utf8_path");
            return Ok(false);
        };
        let path = checked_result_path(self.workspace, path)?;
        if !self.admit(&path)? {
            return Ok(false);
        }
        if binary_end {
            // rg can publish text matches before reporting the NUL offset in
            // its end event. They are not a complete text file.
            self.reasons.insert("binary_content");
            self.matches.retain(|record| record["path"] != path);
            self.context.retain(|record| record["path"] != path);
            if self.matches.len() < self.options.max_matches {
                self.last_match = None;
                return Ok(false);
            }
            return Ok(self.limit_reached(kind));
        }
        let Some(number) = data["line_number"].as_u64() else {
            return Err(ToolError::InvalidArguments(
                "ripgrep line number missing".into(),
            ));
        };
        if kind == "match" && self.last_match.is_some() {
            return Ok(self.limit_reached(kind));
        }
        if kind == "context"
            && let Some((last_path, last_line)) = &self.last_match
            && (*last_path != path || number > last_line + self.options.context as u64)
        {
            // Leading context of a match beyond the limit.
            return Ok(false);
        }
        let Some(text) = data["lines"]["text"].as_str() else {
            self.reasons.insert("non_utf8_content");
            return Ok(false);
        };
        if text.contains('\0') {
            self.reasons.insert("binary_content");
            return Ok(false);
        }
        let text = text.trim_end_matches(['\r', '\n']);
        if text.len() > MAX_LINE_BYTES {
            self.reasons.insert("line_bytes");
        }
        let record = json!({"path":path,"line":number,"text":bounded_text(text,MAX_LINE_BYTES)});
        if kind == "match" {
            self.matches.push(record);
            if self.matches.len() >= self.options.max_matches {
                self.last_match = Some((path, number));
            }
        } else if self.context.len()
            < self
                .options
                .max_matches
                .saturating_mul(self.options.context * 2)
        {
            self.context.push(record);
        } else {
            self.reasons.insert("context_limit");
        }
        Ok(false)
    }
    /// With the match limit reached, stop at the first event that is not
    /// trailing context of the last admitted match.
    fn limit_reached(&mut self, kind: &str) -> bool {
        if self.last_match.is_none() {
            return false;
        }
        if kind == "match" || kind == "end" {
            self.reasons.insert("match_limit");
        }
        true
    }
    /// Per-result safety: inside the search root and workspace, a regular
    /// non-symlink file, and never a host-private output inode.
    fn admit(&mut self, path: &str) -> Result<bool, ToolError> {
        if let Some(admitted) = self.admitted.get(path) {
            return Ok(*admitted);
        }
        if self.root != Path::new(".") && !Path::new(path).starts_with(&self.root) {
            return Err(ToolError::PathDenied(
                "search result outside search root".into(),
            ));
        }
        let full = self.workspace.join(path);
        let metadata = fs::symlink_metadata(&full)?;
        let admitted = if !metadata.is_file() || metadata.file_type().is_symlink() {
            self.reasons.insert("non_regular_file");
            false
        } else {
            let canonical = full.canonicalize()?;
            if !canonical.starts_with(self.workspace)
                || crate::tools::private_output_path(&canonical)
            {
                return Err(ToolError::PathDenied(path.into()));
            }
            // Hard-link aliases of private output are skipped, never shown.
            !crate::tool_output::private_output_path_inode(&full)?
        };
        self.admitted.insert(path.into(), admitted);
        Ok(admitted)
    }
}
#[derive(Debug)]
pub struct SearchOptions {
    pub query: String,
    pub regex: bool,
    pub glob: Option<String>,
    pub ignore: bool,
    pub hidden: bool,
    pub context: usize,
    pub case_sensitive: bool,
    pub max_matches: usize,
}
impl SearchOptions {
    fn validate(&self) -> Result<(), ToolError> {
        if self.query.is_empty()
            || self.query.len() > 4096
            || self.query.contains('\0')
            || self.context > 10
            || !(1..=100).contains(&self.max_matches)
        {
            return Err(ToolError::InvalidArguments(
                "search arguments exceed limits".into(),
            ));
        }
        if let Some(glob) = &self.glob {
            validate_glob(glob)?;
        }
        Ok(())
    }
}
/// Whether a regex query has no metacharacters, so it matches exactly what
/// the same text matches as a fixed string.
pub fn regex_is_literal(query: &str) -> bool {
    !query.contains([
        '\\', '.', '+', '*', '?', '(', ')', '|', '[', ']', '{', '}', '^', '$',
    ])
}
fn validate_glob(glob: &str) -> Result<(), ToolError> {
    if glob.is_empty() || glob.len() > 4096 || glob.contains(['\0', '\r', '\n']) {
        Err(ToolError::InvalidArguments(
            "glob must be nonempty and at most 4096 bytes".into(),
        ))
    } else {
        Ok(())
    }
}
fn checked_result_path(workspace: &Path, path: &str) -> Result<String, ToolError> {
    let path = Path::new(path);
    if crate::tools::private_output_path(path) {
        return Err(ToolError::PathDenied(path.display().to_string()));
    }
    let relative = if path.is_absolute() {
        path.strip_prefix(workspace)
            .map_err(|_| ToolError::PathDenied(path.display().to_string()))?
    } else {
        path
    };
    if relative.components().any(|part| {
        !matches!(
            part,
            std::path::Component::Normal(_) | std::path::Component::CurDir
        )
    }) {
        return Err(ToolError::PathDenied(path.display().to_string()));
    }
    let relative = relative
        .components()
        .filter(|part| *part != std::path::Component::CurDir)
        .collect::<PathBuf>();
    let text = relative
        .to_str()
        .ok_or_else(|| ToolError::InvalidArguments("non-UTF8 search path".into()))?;
    if text.len() > 4096 || text.contains(['\0', '\r', '\n']) {
        return Err(ToolError::PathDenied(text.into()));
    }
    Ok(text.into())
}
fn bounded_text(text: &str, max: usize) -> &str {
    let mut end = max.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}
