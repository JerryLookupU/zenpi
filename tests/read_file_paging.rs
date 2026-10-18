//! ZE-203: read_file pages by lines (pi semantics) and never splits a character.

use serde_json::{Map, Value, json};
use std::fs;
use tempfile::tempdir;
use zenpi::tools::{ReadFileTool, Tool, ToolContext};

fn read(context: &ToolContext, arguments: Value) -> Value {
    let arguments: Map<String, Value> = arguments.as_object().unwrap().clone();
    ReadFileTool.invoke(context, &arguments).unwrap()
}

#[test]
fn a_two_megabyte_cjk_file_pages_completely_without_utf8_errors() {
    let workspace = tempdir().unwrap();
    let line = "中文内容测试行，用于验证分页读取不会切断多字节字符。\n";
    let body = line.repeat(2 * 1024 * 1024 / line.len() + 1);
    fs::write(workspace.path().join("big.txt"), &body).unwrap();
    let context = ToolContext::new(workspace.path()).unwrap();

    let mut collected = String::new();
    let mut offset = 1u64;
    let mut pages = 0;
    loop {
        let page = read(&context, json!({"path": "big.txt", "offset": offset}));
        let content = page["content"].as_str().unwrap();
        assert!(content.len() <= 50 * 1024, "page exceeds 50 KiB");
        collected.push_str(content);
        pages += 1;
        match page["next_offset"].as_u64() {
            Some(next) => {
                assert_eq!(page["truncated"], true);
                offset = next;
            }
            None => break,
        }
    }
    assert_eq!(collected, body, "pages must reassemble the file exactly");
    assert!(
        pages > 40,
        "a 2 MB file needs many 50 KiB pages, got {pages}"
    );
}

#[test]
fn offset_and_limit_select_lines_and_report_totals() {
    let workspace = tempdir().unwrap();
    let body: String = (1..=10).map(|n| format!("line {n}\n")).collect();
    fs::write(workspace.path().join("ten.txt"), body).unwrap();
    let context = ToolContext::new(workspace.path()).unwrap();
    let page = read(
        &context,
        json!({"path": "ten.txt", "offset": 3, "limit": 2}),
    );
    assert_eq!(page["content"], "line 3\nline 4\n");
    assert_eq!(page["start_line"], 3);
    assert_eq!(page["end_line"], 4);
    assert_eq!(page["total_lines"], 10);
    assert_eq!(page["next_offset"], 5);
    let rest = read(&context, json!({"path": "ten.txt", "offset": 9}));
    assert_eq!(rest["content"], "line 9\nline 10\n");
    assert_eq!(rest["truncated"], false);
    assert!(rest["next_offset"].is_null());
}

#[test]
fn an_overlong_single_line_is_cut_on_a_character_boundary() {
    let workspace = tempdir().unwrap();
    fs::write(workspace.path().join("one.txt"), "字".repeat(30_000)).unwrap();
    let context = ToolContext::new(workspace.path()).unwrap();
    let page = read(&context, json!({"path": "one.txt"}));
    let content = page["content"].as_str().unwrap();
    assert!(content.len() <= 50 * 1024);
    assert!(content.chars().all(|c| c == '字'));
    assert_eq!(page["truncated"], true);
}
