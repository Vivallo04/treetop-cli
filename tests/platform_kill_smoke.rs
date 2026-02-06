use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, Signal, System};
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
        let mut c = Command::new("cmd");
        c.args(["/C", "timeout /T 30 /NOBREAK >NUL"]);
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
    refresh_system(&mut sys);

    let result = kill_process(&sys, pid, Signal::Term);

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
