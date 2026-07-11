use super::GhError;
use anyhow::{Context, Result, anyhow, bail};
use std::io::{self, Read};
use std::path::Path;
use std::process::{Command, ExitStatus, Output, Stdio};
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct GhCommand {
    timeout: Duration,
}

pub(crate) trait CommandRunner: Send + Sync {
    fn run(&self, args: Vec<String>) -> Result<Output>;
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
        self.run_command(Path::new("gh"), args)
    }

    fn run_command(&self, executable: &Path, args: Vec<String>) -> Result<Output> {
        let mut child = Command::new(executable)
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| gh_spawn_error(error, &args))?;
        let stdout = child.stdout.take().context("failed to capture gh stdout")?;
        let stderr = child.stderr.take().context("failed to capture gh stderr")?;
        let stdout_reader = thread::spawn(move || read_pipe(stdout));
        let stderr_reader = thread::spawn(move || read_pipe(stderr));
        let started = Instant::now();

        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    return collect_output(status, stdout_reader, stderr_reader);
                }
                Ok(None) => {}
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    join_readers(stdout_reader, stderr_reader)?;
                    return Err(error).context("failed to poll gh command");
                }
            }

            if started.elapsed() >= self.timeout {
                let _ = child.kill();
                let _ = child.wait();
                join_readers(stdout_reader, stderr_reader)?;
                bail!(GhError::Timeout(format!(
                    "gh command timed out after {}s: gh {}",
                    self.timeout.as_secs(),
                    args.join(" ")
                )));
            }

            thread::sleep(Duration::from_millis(25));
        }
    }
}

impl CommandRunner for GhCommand {
    fn run(&self, args: Vec<String>) -> Result<Output> {
        GhCommand::run(self, args)
    }
}

fn read_pipe(mut pipe: impl Read) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    pipe.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn collect_output(
    status: ExitStatus,
    stdout_reader: JoinHandle<io::Result<Vec<u8>>>,
    stderr_reader: JoinHandle<io::Result<Vec<u8>>>,
) -> Result<Output> {
    let (stdout, stderr) = join_readers(stdout_reader, stderr_reader)?;
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

fn join_readers(
    stdout_reader: JoinHandle<io::Result<Vec<u8>>>,
    stderr_reader: JoinHandle<io::Result<Vec<u8>>>,
) -> Result<(Vec<u8>, Vec<u8>)> {
    let stdout = stdout_reader.join();
    let stderr = stderr_reader.join();
    let stdout = stdout
        .map_err(|_| anyhow!("gh stdout reader thread panicked"))?
        .context("failed to read gh stdout")?;
    let stderr = stderr
        .map_err(|_| anyhow!("gh stderr reader thread panicked"))?
        .context("failed to read gh stderr")?;
    Ok((stdout, stderr))
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
    use std::io::{self, Write};
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

    #[test]
    fn drains_large_stdout_and_stderr_without_deadlocking() {
        let executable = std::env::current_exe().unwrap();
        let output = GhCommand::new(Duration::from_secs(5))
            .run_command(
                &executable,
                vec![
                    "--ignored".into(),
                    "--exact".into(),
                    "github::command::tests::large_pipe_writer".into(),
                    "--nocapture".into(),
                ],
            )
            .unwrap();

        assert!(output.status.success());
        assert!(output.stdout.windows(8).any(|bytes| bytes == b"oooooooo"));
        assert!(output.stderr.windows(8).any(|bytes| bytes == b"eeeeeeee"));
        assert!(output.stdout.len() > 1024 * 1024);
        assert!(output.stderr.len() > 1024 * 1024);
    }

    #[test]
    #[ignore]
    fn large_pipe_writer() {
        let bytes = 2 * 1024 * 1024;
        io::stdout().write_all(&vec![b'o'; bytes]).unwrap();
        io::stderr().write_all(&vec![b'e'; bytes]).unwrap();
    }

    fn output(stderr: &[u8], stdout: &[u8]) -> Output {
        Output {
            status: ExitStatus::from_raw(1),
            stdout: stdout.to_vec(),
            stderr: stderr.to_vec(),
        }
    }
}
