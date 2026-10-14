use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use zenpi::{
    project_workspace::{add_worktree, list_worktrees, remove_worktree},
    tui::{ProjectTabMetadata, TuiState},
};

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
fn repo() -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    git(&root, &["init", "-q"]);
    git(&root, &["config", "user.name", "fixture"]);
    git(&root, &["config", "user.email", "fixture@example.invalid"]);
    fs::write(root.join("tracked.txt"), "original\n").unwrap();
    git(&root, &["add", "tracked.txt"]);
    git(&root, &["commit", "-qm", "initial"]);
    fs::create_dir(root.join(".zenpi-worktrees")).unwrap();
    (temp, root)
}
fn worktree(root: &Path, name: &str) -> PathBuf {
    let path = root.join(".zenpi-worktrees").join(name);
    add_worktree(root, &path, name).unwrap();
    path
}

#[test]
fn dirty_tracked_untracked_and_staged_worktrees_are_never_forced_away() {
    let (_temp, root) = repo();
    let path = worktree(&root, "dirty");
    fs::write(path.join("tracked.txt"), "edited\n").unwrap();
    assert!(remove_worktree(&root, &path).is_err());
    assert_eq!(
        fs::read_to_string(path.join("tracked.txt")).unwrap(),
        "edited\n"
    );
    git(&path, &["checkout", "--", "tracked.txt"]);
    fs::write(path.join("untracked.txt"), "keep\n").unwrap();
    assert!(remove_worktree(&root, &path).is_err());
    assert!(path.join("untracked.txt").exists());
    git(&path, &["add", "untracked.txt"]);
    assert!(remove_worktree(&root, &path).is_err());
    assert_eq!(list_worktrees(&root).unwrap().len(), 2);
}

#[test]
fn locked_worktree_refuses_and_clean_unlocked_removal_preserves_branch() {
    let (_temp, root) = repo();
    let path = worktree(&root, "locked");
    git(&root, &["worktree", "lock", path.to_str().unwrap()]);
    assert!(remove_worktree(&root, &path).is_err());
    assert!(path.exists());
    git(&root, &["worktree", "unlock", path.to_str().unwrap()]);
    remove_worktree(&root, &path).unwrap();
    assert!(!path.exists());
    assert_eq!(list_worktrees(&root).unwrap().len(), 1);
    git(&root, &["show-ref", "--verify", "refs/heads/locked"]);
}

#[test]
fn ignored_local_files_are_not_deleted_by_git_cleanliness_rules() {
    let (_temp, root) = repo();
    fs::write(root.join(".gitignore"), "ignored-local.txt\n").unwrap();
    git(&root, &["add", ".gitignore"]);
    git(&root, &["commit", "-qm", "ignore local files"]);
    let path = worktree(&root, "ignored");
    fs::write(path.join("ignored-local.txt"), "local data\n").unwrap();
    assert!(remove_worktree(&root, &path).is_err());
    assert_eq!(
        fs::read_to_string(path.join("ignored-local.txt")).unwrap(),
        "local data\n"
    );
    fs::remove_file(path.join("ignored-local.txt")).unwrap();
    remove_worktree(&root, &path).unwrap();
}

#[test]
fn main_foreign_unregistered_and_symlink_roots_are_not_deletion_targets() {
    let (_temp, root) = repo();
    let (_other, outside) = repo();
    assert!(remove_worktree(&root, &root).is_err());
    assert!(remove_worktree(&root, &outside).is_err());
    let ordinary = root.join(".zenpi-worktrees/plain");
    fs::create_dir(&ordinary).unwrap();
    fs::write(ordinary.join("keep"), "data").unwrap();
    assert!(remove_worktree(&root, &ordinary).is_err());
    assert!(ordinary.join("keep").exists());
    #[cfg(unix)]
    {
        let alias = root.join(".zenpi-worktrees/alias");
        std::os::unix::fs::symlink(&outside, &alias).unwrap();
        assert!(remove_worktree(&root, &alias).is_err());
        assert!(outside.join("tracked.txt").exists());
        fs::remove_file(alias).unwrap();
        fs::remove_dir_all(root.join(".zenpi-worktrees")).unwrap();
        std::os::unix::fs::symlink(
            outside.join(".zenpi-worktrees"),
            root.join(".zenpi-worktrees"),
        )
        .unwrap();
        assert!(add_worktree(&root, &root.join(".zenpi-worktrees/escape"), "escape").is_err());
        assert!(!outside.join(".zenpi-worktrees/escape").exists());
    }
}

#[test]
fn invalid_branch_destination_and_traversal_fail_before_git_side_effects() {
    let (_temp, root) = repo();
    for name in ["-option", "bad..name", "bad name", "bad\nname"] {
        assert!(add_worktree(&root, &root.join(".zenpi-worktrees/target"), name).is_err());
        assert!(!root.join(".zenpi-worktrees/target").exists());
    }
    assert!(add_worktree(&root, &root.join(".zenpi-worktrees/../escaped"), "escaped").is_err());
    assert!(add_worktree(&root, &root.join("outside"), "outside").is_err());
    let path = worktree(&root, "existing");
    assert!(add_worktree(&root, &path, "other").is_err());
    assert_eq!(list_worktrees(&root).unwrap().len(), 2);
}

#[test]
fn production_tui_keeps_failed_worktree_tab_and_selection_visible() {
    let (_temp, root) = repo();
    let mut view = TuiState::default();
    view.set_active_project_metadata(ProjectTabMetadata {
        cwd: root.display().to_string(),
        ..Default::default()
    });
    view.set_master_busy(true);
    assert!(view.subtab_add_worktree(Some("busy".into())).is_err());
    assert!(!root.join(".zenpi-worktrees/busy").exists());
    view.set_master_busy(false);
    view.subtab_add_worktree(Some("feature".into())).unwrap();
    let before = view.subtabs();
    let selected = view.active_subtab();
    let path = PathBuf::from(&before[selected].root);
    fs::write(path.join("tracked.txt"), "dirty\n").unwrap();
    view.clear_project_checkpoint_dirty();
    assert!(!view.subtab_close(selected));
    assert_eq!(view.subtabs(), before);
    assert_eq!(view.active_subtab(), selected);
    assert!(!view.project_checkpoint_dirty());
    assert!(view.status().contains("worktree close refused"));
    assert!(path.exists());
    git(&path, &["checkout", "--", "tracked.txt"]);
    view.set_master_busy(true);
    assert!(!view.subtab_close(selected));
    assert!(path.exists());
    view.set_master_busy(false);
    assert!(view.subtab_close(selected));
    assert_eq!(view.subtabs().len(), 1);
    assert!(!path.exists());
}
