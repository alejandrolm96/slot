mod common;

use common::{app_playing_in, boot, tmp_root_with_carts};
use slot::wifi::{WifiJob, WifiJobs};
use slot_input::Action;
use slot_store::{write_slot_state, SlotState};

const DT: f32 = 1.0 / 60.0;

#[derive(Clone, Default)]
struct WifiLog(std::sync::Arc<std::sync::Mutex<Vec<WifiJob>>>);

impl WifiLog {
    fn jobs(&self) -> Vec<WifiJob> {
        self.0.lock().expect("wifi log").clone()
    }
}

impl WifiJobs for WifiLog {
    fn ask(&mut self, job: WifiJob) {
        self.0.lock().expect("wifi log").push(job);
    }
}

fn watched(a: &mut slot::app::App) -> WifiLog {
    let log = WifiLog::default();
    a.set_wifi_jobs(Box::new(log.clone()));
    log
}

#[test]
fn a_card_with_wifi_on_joins_the_home_network() {
    let d = tmp_root_with_carts(&["Emerald", "Ruby"]);
    let mut a = boot(d.path());
    let log = watched(&mut a);
    a.update(DT);
    assert_eq!(log.jobs(), vec![WifiJob::Up]);
}

#[test]
fn a_card_with_wifi_off_takes_a_stale_session_down_instead() {
    let d = tmp_root_with_carts(&["Emerald", "Ruby"]);
    write_slot_state(
        d.path(),
        &SlotState {
            wifi: false,
            ..Default::default()
        },
    )
    .unwrap();
    let mut a = boot(d.path());
    let log = watched(&mut a);
    a.update(DT);
    assert_eq!(
        log.jobs(),
        vec![WifiJob::Down],
        "a crash can leave a session behind a card that no longer wants one"
    );
}

#[test]
fn the_network_is_asked_for_once_rather_than_every_frame() {
    let d = tmp_root_with_carts(&["Emerald", "Ruby"]);
    let mut a = boot(d.path());
    let log = watched(&mut a);
    for _ in 0..180 {
        a.update(DT);
    }
    assert_eq!(
        log.jobs(),
        vec![WifiJob::Up],
        "three seconds of frames must not spawn three seconds of processes"
    );
}

#[test]
fn a_shut_lid_lets_the_home_network_go() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    let log = watched(&mut a);
    a.update(DT);
    a.apply(Action::LidClose);
    a.update(DT);
    assert_eq!(log.jobs(), vec![WifiJob::Up, WifiJob::Down]);
}

#[test]
fn an_opened_lid_joins_again() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    let log = watched(&mut a);
    a.update(DT);
    a.apply(Action::LidClose);
    a.update(DT);
    a.apply(Action::LidOpen);
    a.update(DT);
    assert_eq!(log.jobs(), vec![WifiJob::Up, WifiJob::Down, WifiJob::Up]);
}

#[test]
fn an_opened_lid_joins_nothing_when_the_card_wants_no_network() {
    let d = tmp_root_with_carts(&["Emerald", "Ruby"]);
    write_slot_state(
        d.path(),
        &SlotState {
            wifi: false,
            ..Default::default()
        },
    )
    .unwrap();
    let mut a = boot(d.path());
    let log = watched(&mut a);
    a.update(DT);
    a.apply(Action::LidClose);
    a.update(DT);
    a.apply(Action::LidOpen);
    a.update(DT);
    assert_eq!(log.jobs(), vec![WifiJob::Down]);
}

#[test]
fn a_link_session_keeps_the_home_network_out_of_its_way() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    let log = watched(&mut a);
    a.update(DT);
    a.begin_link(0);
    a.update(DT);
    assert_eq!(
        log.jobs(),
        vec![WifiJob::Up, WifiJob::Down],
        "wlan0 and wlan1 are one radio, so the cable cannot share it"
    );
}

#[test]
fn the_home_network_comes_back_after_the_link_ends() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    let log = watched(&mut a);
    a.update(DT);
    a.begin_link(0);
    a.update(DT);
    a.end_link();
    a.update(DT);
    assert_eq!(log.jobs(), vec![WifiJob::Up, WifiJob::Down, WifiJob::Up]);
}
