//! The home network, run through the card's own script.
//!
//! Joining costs about seven seconds of association and DHCP, which is some
//! four hundred frames, so the work happens on a thread of its own and the app
//! only ever posts a job to it.

#[cfg(feature = "device")]
use std::sync::mpsc::{channel, Sender};
#[cfg(feature = "device")]
use std::sync::OnceLock;
#[cfg(any(feature = "device", test))]
use std::{path::Path, process::Command};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WifiJob {
    Up,
    Down,
}

impl WifiJob {
    pub fn verb(self) -> &'static str {
        match self {
            WifiJob::Up => "up",
            WifiJob::Down => "down",
        }
    }
}

pub trait WifiJobs: Send {
    fn ask(&mut self, job: WifiJob);
}

pub struct WifiQueue;

pub fn wifi_jobs() -> Box<dyn WifiJobs> {
    Box::new(WifiQueue)
}

#[cfg(feature = "device")]
impl WifiJobs for WifiQueue {
    fn ask(&mut self, job: WifiJob) {
        let _ = queue().send(job);
    }
}

// One worker, so a job never overtakes the one before it: a Down that passed an
// Up would leave the card joined when it asked to be alone.
#[cfg(feature = "device")]
fn queue() -> &'static Sender<WifiJob> {
    static Q: OnceLock<Sender<WifiJob>> = OnceLock::new();
    Q.get_or_init(|| {
        let (tx, rx) = channel::<WifiJob>();
        std::thread::spawn(move || {
            for job in rx {
                run(job);
            }
        });
        tx
    })
}

#[cfg(any(feature = "device", test))]
fn net_command(root: &Path, verb: &str) -> Command {
    let mut cmd = Command::new("/bin/sh");
    cmd.arg(root.join("System/slotlink.sh"))
        .arg("net")
        .arg(verb);
    cmd
}

#[cfg(feature = "device")]
fn run(job: WifiJob) {
    let root = std::env::var_os("SLOT_ROOT").unwrap_or_else(|| "/mnt/sdcard".into());
    match net_command(Path::new(&root), job.verb()).status() {
        Ok(status) if status.success() => {}
        Ok(status) => eprintln!("slot: wifi: net {} ended {status}", job.verb()),
        Err(e) => eprintln!("slot: wifi: net {} would not start: {e}", job.verb()),
    }
}

#[cfg(not(feature = "device"))]
impl WifiJobs for WifiQueue {
    fn ask(&mut self, _job: WifiJob) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_job_asks_for_its_own_verb() {
        assert_eq!(WifiJob::Up.verb(), "up");
        assert_eq!(WifiJob::Down.verb(), "down");
    }

    #[test]
    fn asking_for_a_job_is_never_an_error() {
        let mut jobs = wifi_jobs();
        jobs.ask(WifiJob::Up);
        jobs.ask(WifiJob::Down);
    }

    fn argv(cmd: &Command) -> Vec<String> {
        std::iter::once(cmd.get_program())
            .chain(cmd.get_args())
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn a_job_runs_the_cards_script_through_sh() {
        let d = tempfile::tempdir().unwrap();
        let script = d.path().join("System/slotlink.sh");
        assert_eq!(
            argv(&net_command(d.path(), "up")),
            ["/bin/sh", script.to_str().unwrap(), "net", "up"]
        );
    }
}
