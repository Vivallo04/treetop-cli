use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, Signal, System};
use treetop::system::kill::{KillResult, kill_process};

fn refresh_system(sys: &mut System) {
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::everything(),
    );
}

fn spawn_long_lived_child() -> Child {
    #[cfg(windows)]
    let mut cmd = {
        let mut c = Command::new("powershell");
        c.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Start-Sleep -Seconds 30",
        ]);
        c
    };

    #[cfg(not(windows))]
    let mut cmd = {
        let mut c = Command::new("sh");
        c.args(["-c", "sleep 30"]);
        c
    };

    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn child process")
}

fn wait_for_pid(sys: &mut System, pid: u32, timeout: Duration) -> bool {
    let sys_pid = Pid::from_u32(pid);
    let deadline = Instant::now() + timeout;
    loop {
        let pids = [sys_pid];
        sys.refresh_processes_specifics(
            ProcessesToUpdate::Some(&pids),
            true,
            ProcessRefreshKind::everything(),
        );
        if sys.process(sys_pid).is_some() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn kill_nonexistent_pid_returns_not_found() {
    let mut sys = System::new();
    refresh_system(&mut sys);

    let result = kill_process(&sys, u32::MAX, Signal::Term);
    assert!(matches!(result, KillResult::NotFound(_)));
}

#[test]
fn kill_spawned_child_terminates() {
    let mut child = spawn_long_lived_child();
    let pid = child.id();

    let mut sys = System::new();
    if !wait_for_pid(&mut sys, pid, Duration::from_secs(3)) {
        let _ = child.kill();
        panic!("child process PID {pid} was not observed by sysinfo before kill attempt");
    }

    let signal = if cfg!(windows) {
        Signal::Kill
    } else {
        Signal::Term
    };
    let mut result = kill_process(&sys, pid, signal);
    if matches!(result, KillResult::NotFound(_) | KillResult::Failed(_)) {
        thread::sleep(Duration::from_millis(100));
        refresh_system(&mut sys);
        result = kill_process(&sys, pid, Signal::Kill);
    }

    match result {
        KillResult::Success(_, _) => {
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                match child.try_wait() {
                    Ok(Some(_)) => break,
                    Ok(None) if Instant::now() < deadline => {
                        thread::sleep(Duration::from_millis(50));
                    }
                    Ok(None) => {
                        let _ = child.kill();
                        panic!("child process did not exit before timeout");
                    }
                    Err(err) => {
                        let _ = child.kill();
                        panic!("failed waiting for child exit: {err}");
                    }
                }
            }
        }
        KillResult::Failed(err) => {
            let _ = child.kill();
            panic!("kill_process reported failure: {err}");
        }
        KillResult::NotFound(_) => {
            let _ = child.kill();
            panic!("child process not found in sysinfo snapshot");
        }
    }
}
