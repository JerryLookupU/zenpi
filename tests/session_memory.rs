//! ZE-305: a reopened journal keeps each record as its validated line, not a
//! `Value` tree, so resident memory stays a small multiple of the file. Own
//! test binary: the counting allocator is process-wide.
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicIsize, Ordering};
use zenpi::{
    core::{Turn, TurnRole},
    session::SessionStore,
};

struct Counting;
static LIVE: AtomicIsize = AtomicIsize::new(0);

// SAFETY: delegates every call to `System` and only counts sizes.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        LIVE.fetch_add(layout.size() as isize, Ordering::Relaxed);
        // SAFETY: forwarded unchanged to the system allocator.
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size() as isize, Ordering::Relaxed);
        // SAFETY: `pointer` came from `alloc` above with this layout.
        unsafe { System.dealloc(pointer, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

#[test]
fn reopened_session_stays_a_small_multiple_of_its_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("s.jsonl");
    {
        let mut session = SessionStore::open(&path).unwrap();
        for index in 0..1000 {
            let role = if index % 2 == 0 {
                TurnRole::User
            } else {
                TurnRole::Assistant
            };
            let text = format!("message {index} {}", "lorem ipsum dolor ".repeat(8));
            session
                .append_turn(Turn::new(format!("t{index}"), role, text))
                .unwrap();
            session
                .append_event(
                    serde_json::json!({"type": "note", "i": index, "text": "x".repeat(60)}),
                )
                .unwrap();
        }
    }
    let file = std::fs::metadata(&path).unwrap().len() as f64;
    let before = LIVE.load(Ordering::Relaxed);
    let session = SessionStore::open_existing(&path).unwrap();
    let resident = (LIVE.load(Ordering::Relaxed) - before) as f64;
    assert_eq!(session.records().len(), 2001);
    // Was 6.5x with a `Value` per record; events still keep a `Value` each.
    assert!(
        resident < 3.5 * file,
        "{resident} bytes for a {file} byte file"
    );
}
