use super::GhError;
use anyhow::{Context, Result, anyhow, bail};
use std::io;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct GhCommand {
    timeout: Duration,
}

impl GhCommand {
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
    }

    pub fn run<I, S>(&self, args: I) -> Result<Output>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let args: Vec<String> = args
            .into_iter()
            .map(|arg| arg.as_ref().to_owned())
            .collect();
        let mut child = Command::new("gh")
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| gh_spawn_error(error, &args))?;
        let started = Instant::now();

        loop {
            if child
                .try_wait()
                .context("failed to poll gh command")?
                .is_some()
            {
                return child
                    .wait_with_output()
                    .context("failed to collect gh command output");
            }

            if started.elapsed() >= self.timeout {
                let _ = child.kill();
                let _ = child.wait();
                bail!(GhError::Timeout(format!(
                    "gh command timed out after {}s: gh {}",
                    self.timeout.as_secs(),
                    args.join(" ")
                )));
            }

            thread::sleep(Duration::from_millis(25));
        }
    }

    pub fn version(&self) -> Option<String> {
        let output = self.run(["--version"]).ok()?;
        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout.lines().next().map(str::trim).map(str::to_owned)
    }
}

pub fn command_error(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();

    if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        format!("command exited with {}", output.status)
    }
}

fn gh_spawn_error(error: io::Error, args: &[String]) -> anyhow::Error {
    if error.kind() == io::ErrorKind::NotFound {
        anyhow!(GhError::Missing(format!(
            "GitHub CLI `gh` was not found on PATH while running: gh {}",
            args.join(" ")
        )))
    } else {
        anyhow!(GhError::Command(format!(
            "failed to run gh {}: {error}",
            args.join(" ")
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;
    use std::process::{ExitStatus, Output};

    #[test]
    fn command_error_prefers_stderr_then_stdout_then_status() {
        assert_eq!(command_error(&output(b"details", b"ignored")), "details");
        assert_eq!(
            command_error(&output(b"", b"stdout details")),
            "stdout details"
        );
        assert!(command_error(&output(b"", b"")).contains("command exited with"));
    }

    fn output(stderr: &[u8], stdout: &[u8]) -> Output {
        Output {
            status: ExitStatus::from_raw(1),
            stdout: stdout.to_vec(),
            stderr: stderr.to_vec(),
        }
    }
}
