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
fn show_prints_transcript() {
    let root = setup();
    Command::cargo_bin("csess")
        .unwrap()
        .args([
            "/home/sibin/my-works/demo",
            "--show",
            "11111111",
            "--projects-dir",
            root.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("## user"))
        .stdout(predicate::str::contains("Build the thing"));
}

#[test]
fn show_json_has_messages_with_timestamps() {
    let root = setup();
    let out = Command::cargo_bin("csess")
        .unwrap()
        .args([
            "/home/sibin/my-works/demo",
            "--show",
            "11111111",
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
    assert_eq!(v["session_id"], "11111111-aaaa-bbbb-cccc-dddddddddddd");
    assert_eq!(v["messages"][0]["role"], "user");
    assert_eq!(v["messages"][0]["content"], "Build the thing");
    assert_eq!(v["messages"][0]["timestamp"], "2026-06-16T01:00:00Z");
}

#[test]
fn show_limit_returns_last_n_messages() {
    let root = tempfile::tempdir().unwrap();
    let proj = root.path().join("-home-sibin-my-works-demo");
    fs::create_dir_all(&proj).unwrap();
    let mut lines = String::new();
    for i in 0..5 {
        lines.push_str(&format!(
            "{{\"type\":\"user\",\"cwd\":\"/home/sibin/my-works/demo\",\"timestamp\":\"2026-06-16T01:0{i}:00.000Z\",\"message\":{{\"role\":\"user\",\"content\":\"msg{i}\"}}}}\n"
        ));
    }
    fs::write(
        proj.join("11111111-aaaa-bbbb-cccc-dddddddddddd.jsonl"),
        lines,
    )
    .unwrap();

    let out = Command::cargo_bin("csess")
        .unwrap()
        .args([
            "/home/sibin/my-works/demo",
            "--show",
            "11111111",
            "--json",
            "-n",
            "2",
            "--projects-dir",
            root.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    let msgs = v["messages"].as_array().unwrap();
    assert_eq!(msgs.len(), 2);
    assert_eq!(msgs[0]["content"], "msg3");
    assert_eq!(msgs[1]["content"], "msg4");
}

#[test]
fn show_before_returns_messages_older_than_cursor() {
    let root = tempfile::tempdir().unwrap();
    let proj = root.path().join("-home-sibin-my-works-demo");
    fs::create_dir_all(&proj).unwrap();
    let mut lines = String::new();
    for i in 0..5 {
        lines.push_str(&format!(
            "{{\"type\":\"user\",\"uuid\":\"u{i}\",\"cwd\":\"/home/sibin/my-works/demo\",\"timestamp\":\"2026-06-16T01:0{i}:00.000Z\",\"message\":{{\"role\":\"user\",\"content\":\"msg{i}\"}}}}\n"
        ));
    }
    fs::write(
        proj.join("11111111-aaaa-bbbb-cccc-dddddddddddd.jsonl"),
        lines,
    )
    .unwrap();

    // cursor at msg3 → only the strictly-older msg0..=msg2 remain; -n 2 keeps the last two of those
    let out = Command::cargo_bin("csess")
        .unwrap()
        .args([
            "/home/sibin/my-works/demo",
            "--show",
            "11111111",
            "--before",
            "u3",
            "--json",
            "-n",
            "2",
            "--projects-dir",
            root.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    let msgs = v["messages"].as_array().unwrap();
    assert_eq!(msgs.len(), 2);
    assert_eq!(msgs[0]["content"], "msg1");
    assert_eq!(msgs[1]["content"], "msg2");
}

#[test]
fn show_no_match_exits_2() {
    let root = setup();
    Command::cargo_bin("csess")
        .unwrap()
        .args([
            "/home/sibin/my-works/demo",
            "--show",
            "nope-no-such",
            "--projects-dir",
            root.path().to_str().unwrap(),
        ])
        .assert()
        .code(2);
}

#[test]
fn missing_projects_dir_exits_2() {
    Command::cargo_bin("csess")
        .unwrap()
        .args(["/tmp/whatever", "--projects-dir", "/nonexistent/path/xyz"])
        .assert()
        .code(2);
}
