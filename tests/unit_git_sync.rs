use aura_sync::{
    git_checkout_branch, git_commit, git_create_branch, git_diff, git_discard_file, git_init,
    git_pull, git_push, git_stage, git_status, git_unstage, GitFileStatusKind,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

struct TempDirGuard(PathBuf);

impl TempDirGuard {
    fn new(prefix: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("{prefix}_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("create temp dir");
        Self(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn configure_git_identity(dir: &Path) {
    let st1 = Command::new("git")
        .current_dir(dir)
        .args(["config", "user.name", "Aura Test"])
        .status()
        .expect("git config user.name");
    assert!(st1.success());
    let st2 = Command::new("git")
        .current_dir(dir)
        .args(["config", "user.email", "test@aura.local"])
        .status()
        .expect("git config user.email");
    assert!(st2.success());
}

#[test]
fn test_git_init_status_diff_stage_unstage_commit_and_branch() {
    let tmp = TempDirGuard::new("aura_git_local");
    let repo = tmp.path();

    // Non-repo reports is_repo = false cleanly
    let pre = git_status(repo).expect("status on non-repo");
    assert!(!pre.is_repo);

    git_init(repo).expect("git init");
    configure_git_identity(repo);

    let file_path = repo.join("login.aura");
    fs::write(&file_path, "name = \"Login\"\nmethod = \"POST\"\n").expect("write aura file");

    let snap = git_status(repo).expect("status with untracked file");
    assert!(snap.is_repo);
    assert_eq!(snap.unstaged.len(), 1);
    assert_eq!(snap.unstaged[0].path, "login.aura");
    assert_eq!(snap.unstaged[0].kind, GitFileStatusKind::Untracked);

    // Synthetic diff preview for untracked file
    let untracked_diff = git_diff(repo, "login.aura", false).expect("diff untracked");
    assert!(untracked_diff.contains("+++ b/login.aura"));
    assert!(untracked_diff.contains("+name = \"Login\""));

    // Stage then unstage on unborn HEAD
    git_stage(repo, Some("login.aura")).expect("stage file");
    let staged_snap = git_status(repo).expect("status after stage");
    assert_eq!(staged_snap.staged.len(), 1);
    assert_eq!(staged_snap.staged[0].kind, GitFileStatusKind::Added);

    git_unstage(repo, Some("login.aura")).expect("unstage on unborn head");
    let unstaged_snap = git_status(repo).expect("status after unstage");
    assert!(unstaged_snap.staged.is_empty());
    assert_eq!(unstaged_snap.unstaged.len(), 1);

    // Commit with auto-staging enabled
    git_commit(repo, "Initial login request", true).expect("initial commit");
    let clean_snap = git_status(repo).expect("status after commit");
    assert_eq!(clean_snap.total_changes(), 0);
    assert_eq!(clean_snap.recent_commits.len(), 1);
    assert_eq!(clean_snap.recent_commits[0].message, "Initial login request");
    let default_branch = clean_snap.branch.clone();

    // Modify file and inspect working diff + discard
    fs::write(&file_path, "name = \"Login Updated\"\nmethod = \"POST\"\n").unwrap();
    let mod_diff = git_diff(repo, "login.aura", false).expect("modified diff");
    assert!(mod_diff.contains("-name = \"Login\""));
    assert!(mod_diff.contains("+name = \"Login Updated\""));

    git_discard_file(repo, "login.aura", false).expect("discard edits");
    assert_eq!(
        fs::read_to_string(&file_path).unwrap().replace("\r\n", "\n"),
        "name = \"Login\"\nmethod = \"POST\"\n"
    );

    // Branch creation & checkout switching
    git_create_branch(repo, "feature/auth").expect("create branch");
    let feat_snap = git_status(repo).expect("status on feature branch");
    assert_eq!(feat_snap.branch, "feature/auth");
    assert!(feat_snap.branches.iter().any(|b| b.name == default_branch));
    assert!(feat_snap
        .branches
        .iter()
        .any(|b| b.name == "feature/auth" && b.is_current));

    git_checkout_branch(repo, &default_branch).expect("checkout default branch");
    let back_snap = git_status(repo).expect("status after switch back");
    assert_eq!(back_snap.branch, default_branch);
}

#[test]
fn test_git_push_and_pull_against_local_bare_remote() {
    let bare = TempDirGuard::new("aura_git_bare");
    let alice = TempDirGuard::new("aura_git_alice");
    let bob = TempDirGuard::new("aura_git_bob");

    let st_bare = Command::new("git")
        .arg("init")
        .arg("--bare")
        .arg(bare.path())
        .status()
        .expect("git init --bare");
    assert!(st_bare.success());

    // Alice clones and seeds first commit
    let st_clone_a = Command::new("git")
        .arg("clone")
        .arg(bare.path())
        .arg(alice.path())
        .status()
        .expect("clone alice");
    assert!(st_clone_a.success());
    configure_git_identity(alice.path());

    fs::write(alice.path().join("users.aura"), "name = \"Users\"\n").unwrap();
    git_commit(alice.path(), "Add users endpoint", true).expect("alice commit");
    git_push(alice.path()).expect("alice push");

    // Bob clones, then Alice pushes a second commit, then Bob pulls via git_pull
    let st_clone_b = Command::new("git")
        .arg("clone")
        .arg(bare.path())
        .arg(bob.path())
        .status()
        .expect("clone bob");
    assert!(st_clone_b.success());
    configure_git_identity(bob.path());

    fs::write(alice.path().join("orders.aura"), "name = \"Orders\"\n").unwrap();
    git_commit(alice.path(), "Add orders endpoint", true).expect("alice commit 2");
    git_push(alice.path()).expect("alice push 2");

    let pull_out = git_pull(bob.path()).expect("bob pull");
    assert!(!pull_out.is_empty());
    assert!(bob.path().join("orders.aura").exists());

    let bob_snap = git_status(bob.path()).expect("bob status");
    assert!(bob_snap.has_remote);
    assert_eq!(bob_snap.recent_commits.len(), 2);
}
