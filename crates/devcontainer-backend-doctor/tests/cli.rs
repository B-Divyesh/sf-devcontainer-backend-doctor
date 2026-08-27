use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn json_cli_matches_documented_contract() {
    let directory = tempdir().unwrap();
    fs::write(directory.path().join("compose.yml"), "services:\n  app:\n    image: example\n    volumes:\n      - /var/run/docker.sock:/var/run/docker.sock\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_devcontainer-backend-doctor"))
        .args([
            "check",
            directory.path().to_str().unwrap(),
            "--backend",
            "apple-container",
            "--json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["backend"], "apple-container");
    assert!(
        json["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| finding["rule_id"] == "MOUNT-DOCKER-SOCKET")
    );
}

#[test]
fn warning_threshold_is_enforced() {
    let directory = tempdir().unwrap();
    fs::write(
        directory.path().join("compose.yml"),
        "services:\n  app:\n    image: example\n    privileged: true\n",
    )
    .unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_devcontainer-backend-doctor"))
        .args([
            "check",
            directory.path().to_str().unwrap(),
            "--backend",
            "orbstack",
            "--fail-on",
            "warning",
        ])
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(1));
}
