//! Base system prompt and project context discovery (ZE-230).
//!
//! Mirrors pi's `buildSystemPrompt`: a concise role and tool guide, optional
//! `SYSTEM.md` replacement and `APPEND_SYSTEM.md` suffix, the merged
//! `AGENTS.md` (or `CLAUDE.md`) context files, and one environment line.
//! Every file read is bounded; unreadable files are skipped and reported as
//! diagnostics instead of being guessed at.

use crate::tools::ToolDefinition;
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

/// Total bytes of merged context-file content included in the prompt.
pub const MAX_CONTEXT_BYTES: usize = 64 * 1024;
/// Bytes read from one `SYSTEM.md` / `APPEND_SYSTEM.md`.
pub const MAX_SYSTEM_FILE_BYTES: usize = 64 * 1024;
/// Context file names probed in each directory, in priority order.
pub const CONTEXT_FILE_NAMES: [&str; 2] = ["AGENTS.md", "CLAUDE.md"];
/// Project configuration directory holding `SYSTEM.md` overrides.
pub const PROJECT_CONFIG_DIR: &str = ".zenpi";
/// Marker appended where context or system text was cut at its byte cap.
pub const TRUNCATION_MARKER: &str = "[context truncated: byte cap reached]";

/// Inputs fixed by the host for one prompt build.
#[derive(Debug, Clone)]
pub struct PromptInputs<'a> {
    /// Directory the agent works in (the tool workspace root).
    pub cwd: &'a Path,
    /// Global zenpi directory (`$ZENPI_HOME`, default `~/.zenpi`).
    pub global_dir: Option<&'a Path>,
    /// Tools advertised to the model for this request.
    pub tools: &'a [ToolDefinition],
    /// UTC calendar date, `YYYY-MM-DD`.
    pub date: String,
}

/// One discovered context file after the byte cap was applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextFile {
    pub path: PathBuf,
    pub content: String,
    pub truncated: bool,
}

/// Assembled prompt plus the facts a host may want to surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemPrompt {
    pub text: String,
    pub context_files: Vec<ContextFile>,
    /// Human-readable notices for skipped or truncated files.
    pub diagnostics: Vec<String>,
}

/// Today's UTC date as `YYYY-MM-DD`.
pub fn today_utc() -> String {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| {
            u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
        });
    let mut stamp = crate::b3::unix_ms_to_rfc3339(millis);
    stamp.truncate(10);
    stamp
}

/// Nearest ancestor of `cwd` (inclusive) containing `.git`; `cwd` otherwise.
pub fn workspace_root(cwd: &Path) -> PathBuf {
    cwd.ancestors()
        .find(|dir| dir.join(".git").exists())
        .unwrap_or(cwd)
        .to_path_buf()
}

/// Directories from the workspace root down to `cwd`, root first.
fn project_dirs(cwd: &Path) -> Vec<PathBuf> {
    let root = workspace_root(cwd);
    let mut dirs: Vec<PathBuf> = cwd
        .ancestors()
        .take_while(|dir| dir.starts_with(&root))
        .map(Path::to_path_buf)
        .collect();
    dirs.reverse();
    dirs
}

enum Bounded {
    Missing,
    Text { content: String, truncated: bool },
    Unreadable(String),
}

/// Read at most `cap` bytes of UTF-8 text, cutting on a char boundary.
fn read_bounded(path: &Path, cap: usize) -> Bounded {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Bounded::Missing,
        Err(error) => return Bounded::Unreadable(error.to_string()),
        Ok(_) => {}
    }
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) => return Bounded::Unreadable(error.to_string()),
    };
    match file.metadata() {
        Ok(metadata) if !metadata.is_file() => {
            return Bounded::Unreadable("not a regular file".into());
        }
        Err(error) => return Bounded::Unreadable(error.to_string()),
        Ok(_) => {}
    }
    let mut bytes = Vec::new();
    let limit = u64::try_from(cap).unwrap_or(u64::MAX).saturating_add(4);
    if let Err(error) = file.take(limit).read_to_end(&mut bytes) {
        return Bounded::Unreadable(error.to_string());
    }
    let truncated = bytes.len() > cap;
    if truncated {
        bytes.truncate(cap);
        // Drop a partial trailing code point (at most three bytes).
        for _ in 0..4 {
            if std::str::from_utf8(&bytes).is_ok() {
                break;
            }
            bytes.pop();
        }
    }
    match String::from_utf8(bytes) {
        Ok(content) => Bounded::Text { content, truncated },
        Err(_) => Bounded::Unreadable("not valid UTF-8".into()),
    }
}

/// `AGENTS.md`, else `CLAUDE.md`, from one directory (pi's fallback order:
/// an unreadable `AGENTS.md` also falls through to `CLAUDE.md`).
fn context_file_in(dir: &Path, cap: usize, diagnostics: &mut Vec<String>) -> Option<ContextFile> {
    for name in CONTEXT_FILE_NAMES {
        let path = dir.join(name);
        match read_bounded(&path, cap) {
            Bounded::Missing => {}
            Bounded::Unreadable(reason) => {
                diagnostics.push(format!("skipped context file {}: {reason}", path.display()));
            }
            Bounded::Text { content, truncated } => {
                return Some(ContextFile {
                    path,
                    content,
                    truncated,
                });
            }
        }
    }
    None
}

/// Merge context files global → workspace root → … → cwd under one total
/// byte cap. Files past the cap are truncated or omitted with a diagnostic.
pub fn discover_context_files(
    global_dir: Option<&Path>,
    cwd: &Path,
    diagnostics: &mut Vec<String>,
) -> Vec<ContextFile> {
    let mut dirs: Vec<PathBuf> = global_dir.map(Path::to_path_buf).into_iter().collect();
    for dir in project_dirs(cwd) {
        if !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }
    let mut files = Vec::new();
    let mut remaining = MAX_CONTEXT_BYTES;
    for dir in dirs {
        if remaining == 0 {
            for name in CONTEXT_FILE_NAMES {
                let path = dir.join(name);
                if path.is_file() {
                    diagnostics.push(format!(
                        "omitted context file {}: {MAX_CONTEXT_BYTES}-byte context cap reached",
                        path.display()
                    ));
                    break;
                }
            }
            continue;
        }
        if let Some(file) = context_file_in(&dir, remaining, diagnostics) {
            if file.truncated {
                diagnostics.push(format!(
                    "truncated context file {} at the {MAX_CONTEXT_BYTES}-byte context cap",
                    file.path.display()
                ));
                remaining = 0;
            } else {
                remaining -= file.content.len();
            }
            files.push(file);
        }
    }
    files
}

/// First readable `name` in cwd's `.zenpi/` up to the workspace root, then the
/// global directory (project overrides global, as in pi).
fn system_file(
    name: &str,
    global_dir: Option<&Path>,
    cwd: &Path,
    diagnostics: &mut Vec<String>,
) -> Option<String> {
    let mut candidates: Vec<PathBuf> = project_dirs(cwd)
        .into_iter()
        .rev()
        .map(|dir| dir.join(PROJECT_CONFIG_DIR).join(name))
        .collect();
    candidates.extend(global_dir.map(|dir| dir.join(name)));
    for path in candidates {
        match read_bounded(&path, MAX_SYSTEM_FILE_BYTES) {
            Bounded::Missing => {}
            Bounded::Unreadable(reason) => {
                diagnostics.push(format!("skipped {}: {reason}", path.display()));
            }
            Bounded::Text {
                mut content,
                truncated,
            } => {
                if truncated {
                    diagnostics.push(format!(
                        "truncated {} at {MAX_SYSTEM_FILE_BYTES} bytes",
                        path.display()
                    ));
                    content.push('\n');
                    content.push_str(TRUNCATION_MARKER);
                }
                return Some(content);
            }
        }
    }
    None
}

/// Usage line for a builtin tool, stating its limits and how to continue.
fn builtin_snippet(name: &str) -> Option<String> {
    use crate::tools::{
        MAX_COMMAND_OUTPUT_BYTES, MAX_COMMAND_TIMEOUT_MS, MAX_LIST_ENTRIES, MAX_READ_BYTES,
        MAX_READ_LINES, MAX_SEARCH_MATCHES, MAX_WRITE_BYTES,
    };
    let kib = |bytes: usize| bytes / 1024;
    Some(match name {
        "read_file" => format!(
            "Read a UTF-8 workspace file by lines: offset (1-based line) and limit (default and cap {MAX_READ_LINES} lines), at most 50 KiB per call (max_bytes up to {} KiB). A partial result ends with \"[Showing lines a-b of N. Use offset=K to continue]\"; call again with that offset to keep reading.",
            kib(MAX_READ_BYTES)
        ),
        "read_tool_output" => format!(
            "Page through captured command output. Pass the artifact_id and call_id from a \"read_tool_output artifact_id=... call_id=...\" hint, start at offset 0, and continue with offset += length (length <= {} KiB) until the stream ends.",
            kib(crate::tool_output::MAX_PUBLIC_OUTPUT_READ_BYTES)
        ),
        "list_directory" => format!(
            "List one workspace directory (max_entries default 64, cap {MAX_LIST_ENTRIES}); not recursive. Use find for recursive listings."
        ),
        "search_text" => format!(
            "Search file contents with ripgrep; the query is a regex by default (literal=true for fixed strings; glob/context optional). Returns at most max_matches (cap {MAX_SEARCH_MATCHES}); narrow path, glob or query when results are capped."
        ),
        "find" => "Find workspace files by glob. Returns at most limit paths (default 100, cap 256); narrow the pattern or path when capped.".to_owned(),
        "write_file" => format!(
            "Create or overwrite a workspace file atomically (content cap {} KiB). Read existing files before overwriting them.",
            kib(MAX_WRITE_BYTES)
        ),
        "edit_file" => "Replace exactly one occurrence of old with new. Include enough surrounding text to make old unique; read the file first.".to_owned(),
        "run_command" => format!(
            "Run a shell command in the workspace. stdout and stderr are returned together with the exit code; a non-zero exit is an error that still includes the output. timeout_ms is optional with no default (max {} h). You see the last 2000 lines / 50 KiB; each stream keeps its last {} KiB, and when earlier output was dropped the result says \"[stdout truncated: earlier output omitted] Full stdout: read_tool_output artifact_id=... call_id=...\", so page the full output with read_tool_output and offset instead of re-running.",
            MAX_COMMAND_TIMEOUT_MS / 3_600_000,
            kib(MAX_COMMAND_OUTPUT_BYTES)
        ),
        "websearch" => "Search the web for current information; cite the returned URLs.".to_owned(),
        _ => return None,
    })
}

/// First sentence of a tool's own description, bounded for the prompt.
fn description_snippet(definition: &ToolDefinition) -> String {
    let text = definition.description.trim();
    let end = text
        .find(". ")
        .map_or(text.len(), |index| index + 1)
        .min(200);
    let mut end = end.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_owned()
}

fn base_prompt(tools: &[ToolDefinition]) -> String {
    let has = |name: &str| tools.iter().any(|tool| tool.name == name);
    let tool_lines = if tools.is_empty() {
        "(none)".to_owned()
    } else {
        tools
            .iter()
            .map(|tool| {
                let snippet =
                    builtin_snippet(&tool.name).unwrap_or_else(|| description_snippet(tool));
                format!("- {}: {snippet}", tool.name)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let mut guidelines = Vec::new();
    if has("run_command") && (has("search_text") || has("find") || has("list_directory")) {
        guidelines.push(
            "Prefer search_text/find/list_directory over run_command for exploring files (bounded, respects .gitignore)",
        );
    }
    if has("edit_file") || has("write_file") {
        guidelines.push("Read a file before editing it; prefer edit_file for small changes");
    }
    if !tools.is_empty() {
        guidelines.push(
            "Every tool result is bounded. When a result says it was truncated or shows a continuation hint (max_bytes, offset, artifact_id), follow that hint instead of repeating the same call",
        );
    }
    guidelines.push("Be concise in your responses");
    guidelines.push("Show file paths clearly when working with files");
    let guidelines = guidelines
        .iter()
        .map(|line| format!("- {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "You are an expert coding assistant operating inside zenpi, a coding agent harness. You help users by reading files, executing commands, editing code, and writing new files.\n\nAvailable tools:\n{tool_lines}\n\nIn addition to the tools above, you may have access to other custom tools depending on the project.\n\nGuidelines:\n{guidelines}"
    )
}

/// Build the complete system prompt for one request.
pub fn build(inputs: &PromptInputs<'_>) -> SystemPrompt {
    let mut diagnostics = Vec::new();
    let mut text = system_file("SYSTEM.md", inputs.global_dir, inputs.cwd, &mut diagnostics)
        .unwrap_or_else(|| base_prompt(inputs.tools));
    if let Some(append) = system_file(
        "APPEND_SYSTEM.md",
        inputs.global_dir,
        inputs.cwd,
        &mut diagnostics,
    ) {
        text.push_str("\n\n");
        text.push_str(&append);
    }
    let context_files = discover_context_files(inputs.global_dir, inputs.cwd, &mut diagnostics);
    if !context_files.is_empty() {
        text.push_str("\n\n# Project Context\n\nProject-specific instructions and guidelines:\n");
        for file in &context_files {
            text.push_str(&format!("\n## {}\n\n{}", file.path.display(), file.content));
            if file.truncated {
                if !file.content.ends_with('\n') {
                    text.push('\n');
                }
                text.push_str(TRUNCATION_MARKER);
            }
            text.push('\n');
        }
    }
    text.push_str(&format!(
        "\nEnvironment: cwd={}; date={} (UTC); os={}-{}",
        inputs.cwd.display(),
        inputs.date,
        std::env::consts::OS,
        std::env::consts::ARCH
    ));
    SystemPrompt {
        text,
        context_files,
        diagnostics,
    }
}
