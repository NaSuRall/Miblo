use anyhow::{Context, Result};
use std::{
    io::{BufRead, BufReader, Read},
    process::{Child, Command, ExitStatus, Stdio},
    sync::mpsc::SyncSender,
    thread,
    time::{Duration, Instant},
};

pub struct Process {
    child: Child,
    pub label: String,
}

impl Process {
    pub fn spawn(mut command: Command, label: &str, sender: SyncSender<String>) -> Result<Self> {
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let mut child = command
            .spawn()
            .with_context(|| trf!("Cannot start {label}", "Impossible de lancer {label}"))?;
        if let Some(stdout) = child.stdout.take() {
            stream(stdout, sender.clone(), label.into());
        }
        if let Some(stderr) = child.stderr.take() {
            stream(stderr, sender, label.into());
        }
        Ok(Self {
            child,
            label: label.into(),
        })
    }
    pub fn poll(&mut self) -> Result<Option<ExitStatus>> {
        Ok(self.child.try_wait()?)
    }
    pub fn stop(&mut self) {
        if self.child.try_wait().ok().flatten().is_some() {
            return;
        }
        #[cfg(unix)]
        {
            // This child owns a dedicated process group created in spawn(). Kill
            // that group so cargo's API child cannot outlive the workspace.
            if let Ok(pid) = i32::try_from(self.child.id()) {
                unsafe {
                    libc::killpg(pid, libc::SIGTERM);
                }
            }
        }
        #[cfg(windows)]
        {
            let _ = Command::new("taskkill")
                .args(["/PID", &self.child.id().to_string(), "/T", "/F"])
                .output();
        }
        let deadline = Instant::now() + Duration::from_secs(1);
        while Instant::now() < deadline {
            if self.child.try_wait().ok().flatten().is_some() {
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }
        #[cfg(unix)]
        {
            if let Ok(pid) = i32::try_from(self.child.id()) {
                unsafe {
                    libc::killpg(pid, libc::SIGKILL);
                }
            }
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        self.stop();
    }
}

fn stream(reader: impl Read + Send + 'static, sender: SyncSender<String>, label: String) {
    thread::spawn(move || {
        let mut reader = BufReader::new(reader);
        loop {
            let mut bytes = Vec::new();
            match reader.by_ref().take(8192).read_until(b'\n', &mut bytes) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let line = String::from_utf8_lossy(&bytes)
                        .chars()
                        .filter(|c| !c.is_control() || *c == '\t')
                        .collect::<String>();
                    // The process must not block if the UI is paused in an editor.
                    let _ = sender.try_send(format!("[{label}] {line}"));
                }
            }
        }
    });
}
