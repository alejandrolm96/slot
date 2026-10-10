mod common;

use common::{app_playing_in, boot, tmp_root_with_carts};
use slot::wifi::{WifiJob, WifiJobs};
use slot_input::Action;
use slot_store::{write_slot_state, SlotState};
use slot_ui::WifiState;

const DT: f32 = 1.0 / 60.0;

#[derive(Clone, Default)]
struct WifiLog {
    jobs: std::sync::Arc<std::sync::Mutex<Vec<WifiJob>>>,
    state: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl WifiLog {
    fn jobs(&self) -> Vec<WifiJob> {
        self.jobs.lock().expect("wifi log").clone()
    }

    fn report(&self, state: WifiState) {
        self.state
            .store(state.index(), std::sync::atomic::Ordering::SeqCst);
    }
}

impl WifiJobs for WifiLog {
    fn ask(&mut self, job: WifiJob) {
        self.jobs.lock().expect("wifi log").push(job);
    }

    fn state(&self) -> WifiState {
        WifiState::from_index(self.state.load(std::sync::atomic::Ordering::SeqCst))
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

#[test]
fn the_app_reports_what_the_worker_is_doing() {
    let d = tmp_root_with_carts(&["Emerald", "Ruby"]);
    let mut a = boot(d.path());
    let log = watched(&mut a);
    a.update(DT);
    assert_eq!(a.wifi_state(), WifiState::Off, "nothing has happened yet");
    for state in [WifiState::Joining, WifiState::Up, WifiState::Off] {
        log.report(state);
        a.update(DT);
        assert_eq!(
            a.wifi_state(),
            state,
            "the frontend is drawing a network the worker is not on"
        );
    }
}

#[test]
fn the_worker_is_asked_once_a_frame_rather_than_cached_forever() {
    let d = tmp_root_with_carts(&["Emerald", "Ruby"]);
    let mut a = boot(d.path());
    let log = watched(&mut a);
    a.update(DT);
    log.report(WifiState::Joining);
    a.update(DT);
    log.report(WifiState::Up);
    a.update(DT);
    assert_eq!(a.wifi_state(), WifiState::Up);
}

mod shown {
    use super::*;
    use slot_gfx::{Draw, TexId};
    use slot_ui::{icon_box, Icon, Toast, HUD_ICON_PX, OUT_W};

    const WIFI: usize = 12;

    fn faced(a: &mut slot::app::App) {
        a.set_icon_faces((0..Icon::ALL.len()).map(TexId::from_raw).collect());
    }

    fn wifi_draws(a: &slot::app::App) -> Vec<(f32, f32)> {
        let mut out = Vec::new();
        a.draw(&mut out);
        out.iter()
            .filter_map(|d| match *d {
                Draw::Tex { tex, x, y, .. } if tex == TexId::from_raw(WIFI) => Some((x, y)),
                _ => None,
            })
            .collect()
    }

    fn in_the_corner(x: f32, y: f32) -> bool {
        let (w, _) = icon_box(HUD_ICON_PX);
        (x - (OUT_W as f32 - 12.0 - w as f32)).abs() < 0.5 && y < 50.0
    }

    #[test]
    fn the_shelf_shows_a_joining_network_once_in_its_footer() {
        let d = tmp_root_with_carts(&["Emerald", "Ruby"]);
        let mut a = boot(d.path());
        faced(&mut a);
        let log = watched(&mut a);
        log.report(WifiState::Joining);
        a.update(DT);
        let drawn = wifi_draws(&a);
        assert_eq!(
            drawn.len(),
            1,
            "the shelf drew the network {} times: {drawn:?}",
            drawn.len()
        );
        assert!(
            !in_the_corner(drawn[0].0, drawn[0].1),
            "the corner icon doubled up on the footer the shelf already has"
        );
    }

    #[test]
    fn a_game_shows_a_joining_network_in_the_corner() {
        let d = tmp_root_with_carts(&["Emerald"]);
        let mut a = app_playing_in(d.path(), "Emerald");
        faced(&mut a);
        let log = watched(&mut a);
        log.report(WifiState::Joining);
        a.update(DT);
        let drawn = wifi_draws(&a);
        assert_eq!(drawn.len(), 1, "expected one icon, got {drawn:?}");
        assert!(
            in_the_corner(drawn[0].0, drawn[0].1),
            "a game drew the network at {drawn:?}, not in the badge corner"
        );
    }

    #[test]
    fn a_game_says_connected_once_the_icon_has_gone() {
        let d = tmp_root_with_carts(&["Emerald"]);
        let mut a = app_playing_in(d.path(), "Emerald");
        faced(&mut a);
        let log = watched(&mut a);
        log.report(WifiState::Joining);
        a.update(DT);
        assert_eq!(a.toast(), None, "it said connected before it was");
        log.report(WifiState::Up);
        a.update(DT);
        assert_eq!(
            a.toast(),
            Some(Toast::WifiConnected),
            "the icon vanished without a word"
        );
        assert!(
            wifi_draws(&a).is_empty(),
            "a settled network is still sitting on the game"
        );
    }

    #[test]
    fn the_shelf_stays_quiet_because_its_footer_already_said_it() {
        let d = tmp_root_with_carts(&["Emerald", "Ruby"]);
        let mut a = boot(d.path());
        faced(&mut a);
        let log = watched(&mut a);
        log.report(WifiState::Joining);
        a.update(DT);
        log.report(WifiState::Up);
        a.update(DT);
        assert_eq!(
            a.toast(),
            None,
            "a toast on top of the icon that just went solid"
        );
    }
}

mod toggled {
    use super::*;
    use slot_input::{Action, Btn};
    use slot_ui::QuickRow;

    fn press(a: &mut slot::app::App, btn: Btn) {
        a.apply(Action::GbaDown(btn));
        a.apply(Action::GbaUp(btn));
    }

    fn at_the_wifi_row(a: &mut slot::app::App) {
        a.apply(Action::QuickMenu);
        for _ in 0..QuickRow::Wifi.position() {
            press(a, Btn::Down);
        }
        assert_eq!(a.quick_menu(), Some(QuickRow::Wifi));
    }

    #[test]
    fn turning_it_off_in_the_menu_lets_the_network_go() {
        let d = tmp_root_with_carts(&["Emerald", "Ruby"]);
        let mut a = boot(d.path());
        let log = watched(&mut a);
        a.update(DT);
        assert_eq!(log.jobs(), vec![WifiJob::Up]);
        at_the_wifi_row(&mut a);
        press(&mut a, Btn::Right);
        a.update(DT);
        assert_eq!(
            log.jobs(),
            vec![WifiJob::Up, WifiJob::Down],
            "the setting changed but the radio stayed on"
        );
    }

    #[test]
    fn turning_it_back_on_joins_again() {
        let d = tmp_root_with_carts(&["Emerald", "Ruby"]);
        let mut a = boot(d.path());
        let log = watched(&mut a);
        a.update(DT);
        at_the_wifi_row(&mut a);
        press(&mut a, Btn::Right);
        a.update(DT);
        press(&mut a, Btn::Left);
        a.update(DT);
        assert_eq!(log.jobs(), vec![WifiJob::Up, WifiJob::Down, WifiJob::Up]);
    }
}
