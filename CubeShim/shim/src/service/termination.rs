// Copyright (c) 2026 Tencent Inc.
// SPDX-License-Identifier: Apache-2.0

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub(super) const CLEANUP_TIMEOUT: Duration = Duration::from_secs(10);

/// Once terminal cleanup starts, this process must exit even if an embedded
/// VMM API request, a lock, or a synchronous thread join never returns.
/// A Tokio timer cannot enforce that deadline when its workers are blocked.
pub(super) fn arm(terminating: &AtomicBool, timeout: Duration) {
    if terminating.swap(true, Ordering::SeqCst) {
        return;
    }
    if std::thread::Builder::new()
        .name("shim-exit-deadline".into())
        .spawn(move || {
            std::thread::sleep(timeout);
            // Do not run destructors/atexit handlers: VmmInstance::drop can
            // block on the same VMM API or join that prevented cleanup.
            // Exiting the whole process closes the embedded VM's descriptors
            // before Cubelet can observe process death and reclaim resources.
            unsafe { libc::_exit(1) }
        })
        .is_err()
    {
        // Without a watchdog we cannot guarantee reclamation of this terminal
        // shim. abort also terminates all of its embedded VMM threads.
        std::process::abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};
    use std::time::Instant;

    #[test]
    fn deadline_child() {
        let Ok(mode) = std::env::var("CUBE_SHIM_DEADLINE_TEST") else {
            return;
        };
        let terminating = AtomicBool::new(false);
        arm(&terminating, Duration::from_millis(100));
        // A duplicate shutdown must not extend the original deadline.
        arm(&terminating, Duration::from_secs(60));
        assert!(terminating.load(Ordering::SeqCst));
        if mode == "clean" {
            std::process::exit(0);
        }
        // Model the synchronous VMM join in a process with no child VMM.
        let worker = std::thread::spawn(|| loop {
            std::thread::park();
        });
        worker.join().unwrap();
    }

    fn run_child(mode: &str) -> std::process::ExitStatus {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "service::termination::tests::deadline_child"])
            .env("CUBE_SHIM_DEADLINE_TEST", mode)
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                return status;
            }
            if start.elapsed() > Duration::from_secs(5) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("terminal shim did not exit within the deadline");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn repeated_blocked_cleanup_exits_without_child_processes() {
        for _ in 0..10 {
            assert_eq!(run_child("blocked").code(), Some(1));
        }
    }

    #[test]
    fn graceful_exit_precedes_deadline() {
        assert!(run_child("clean").success());
    }
}
