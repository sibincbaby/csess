use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

fn seed(root: &std::path::Path, dir: &str, file: &str, cwd: &str, prompt: &str) {
    let proj = root.join(dir);
    fs::create_dir_all(&proj).unwrap();
    let line = format!(
        "{{\"type\":\"user\",\"cwd\":\"{cwd}\",\"timestamp\":\"2026-06-16T01:00:00.000Z\",\"message\":{{\"role\":\"user\",\"content\":\"{prompt}\"}}}}\n"
    );
    fs::write(proj.join(file), line).unwrap();
}

fn setup() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    seed(
        root.path(),
        "-home-sibin-my-works-demo",
        "11111111-aaaa-bbbb-cccc-dddddddddddd.jsonl",
        "/home/sibin/my-works/demo",
        "Build the thing",
    );
    // dashed sibling: loose pre-filter includes it, cwd verify must drop it
    seed(
        root.path(),
        "-home-sibin-my-works-demo-backup",
        "22222222-aaaa-bbbb-cccc-dddddddddddd.jsonl",
        "/home/sibin/my-works/demo-backup",
        "Sibling session",
    );
    root
}

#[test]
fn lists_sessions_as_table() {
    let root = setup();
    Command::cargo_bin("csess")
        .unwrap()
        .args([
            "/home/sibin/my-works/demo",
            "--projects-dir",
            root.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Build the thing"))
        .stdout(predicate::str::contains("11111111"))
        .stdout(predicate::str::contains("Sibling session").not());
}

#[test]
fn json_output_is_valid() {
    let root = setup();
    let out = Command::cargo_bin("csess")
        .unwrap()
        .args([
            "/home/sibin/my-works/demo",
            "--json",
            "--projects-dir",
            root.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert!(v.is_array());
    assert_eq!(v[0]["name"], "Build the thing");
    assert_eq!(v.as_array().unwrap().len(), 1);
}

#[test]
fn missing_projects_dir_exits_2() {
    Command::cargo_bin("csess")
        .unwrap()
        .args(["/tmp/whatever", "--projects-dir", "/nonexistent/path/xyz"])
        .assert()
        .code(2);
}
