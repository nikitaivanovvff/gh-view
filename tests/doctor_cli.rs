use assert_cmd::Command;
use predicates::prelude::*;

fn doctor_command() -> Command {
    let mut command = Command::cargo_bin("gh-view").expect("binary should build");
    command.env("GH_VIEW_CONFIG", "/path/that/does/not/exist");
    command
}

#[test]
fn mock_doctor_exits_successfully() {
    doctor_command()
        .args(["--mock", "doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Data source: built-in mock data"))
        .stdout(predicate::str::contains("GitHub auth: configured"));
}

#[test]
fn doctor_exits_nonzero_when_gh_is_missing() {
    doctor_command()
        .arg("doctor")
        .env("PATH", "")
        .assert()
        .failure()
        .stdout(predicate::str::contains(
            "GitHub CLI: not found on PATH; install gh before fetching PRs.",
        ));
}

#[cfg(unix)]
#[test]
fn doctor_exits_nonzero_when_gh_is_unauthenticated() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after the Unix epoch")
        .as_nanos();
    let directory = std::env::temp_dir().join(format!("gh-view-doctor-{unique}"));
    fs::create_dir(&directory).expect("temporary directory should be created");
    let gh = directory.join("gh");
    fs::write(
        &gh,
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then\n  printf 'gh version test\\n'\n  exit 0\nfi\nprintf 'not logged in\\n' >&2\nexit 1\n",
    )
    .expect("fake gh should be written");
    let mut permissions = fs::metadata(&gh)
        .expect("fake gh metadata should be readable")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&gh, permissions).expect("fake gh should be executable");

    doctor_command()
        .arg("doctor")
        .env("PATH", &directory)
        .assert()
        .failure()
        .stdout(predicate::str::contains(
            "GitHub CLI: installed but not authenticated",
        ))
        .stdout(predicate::str::contains("Run: gh auth login"))
        .stdout(predicate::str::contains("Details: not logged in"));

    fs::remove_dir_all(directory).expect("temporary directory should be removed");
}
