//! ZE-308: idle DAG waiters cost one stat per poll. Its own test binary so
//! no other test shares the process CPU being measured.
use std::time::Duration;
use tempfile::tempdir;
use zenpi::dag::DagStore;

#[cfg(unix)]
#[test]
fn dag_idle_waiters_never_write_and_stay_cheap() {
    // ZE-308: 2000 waiting workers stat the store; none locks, rewrites or
    // fsyncs it while nothing arrives.
    fn cpu_seconds() -> f64 {
        // SAFETY: `usage` is a zeroed out-parameter getrusage fills in.
        let usage = unsafe {
            let mut usage: libc::rusage = std::mem::zeroed();
            libc::getrusage(libc::RUSAGE_SELF, &mut usage);
            usage
        };
        let time = |value: libc::timeval| value.tv_sec as f64 + value.tv_usec as f64 / 1e6;
        time(usage.ru_utime) + time(usage.ru_stime)
    }
    let dir = tempdir().unwrap();
    let path = dir.path().join("dag.json");
    let store = DagStore::at(&path);
    store.upsert("idle", None).unwrap();
    let modified = || std::fs::metadata(&path).unwrap().modified().unwrap();
    let before = modified();
    let (cpu, wall) = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..2000)
            .map(|_| {
                let store = DagStore::at(&path);
                std::thread::Builder::new()
                    .stack_size(64 * 1024)
                    .spawn_scoped(scope, move || {
                        zenpi::dag::wait_for_message(
                            &store,
                            "idle",
                            Duration::from_secs(3),
                            &|| false,
                        )
                        .unwrap()
                    })
                    .unwrap()
            })
            .collect();
        // Sample after start-up, while every waiter sleeps between polls.
        std::thread::sleep(Duration::from_millis(1000));
        let (cpu, started) = (cpu_seconds(), std::time::Instant::now());
        std::thread::sleep(Duration::from_millis(1500));
        let sample = (cpu_seconds() - cpu, started.elapsed().as_secs_f64());
        for handle in handles {
            assert!(handle.join().unwrap().is_empty());
        }
        sample
    });
    assert_eq!(modified(), before, "an empty wait rewrote the store");
    assert!(cpu / wall < 0.25, "{cpu:.3}s CPU over {wall:.2}s");
}
