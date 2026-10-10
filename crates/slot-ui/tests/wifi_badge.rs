use slot_ui::{wifi_alpha, Icon, WifiState, PULSE_MS};

#[test]
fn an_off_network_draws_nothing_at_all() {
    assert_eq!(wifi_alpha(WifiState::Off, 0), None);
    assert_eq!(wifi_alpha(WifiState::Off, 743), None);
}

#[test]
fn a_joined_network_sits_still_and_solid() {
    for now in [0, 300, PULSE_MS / 2, PULSE_MS, 98_765] {
        assert_eq!(
            wifi_alpha(WifiState::Up, now),
            Some(1.0),
            "a settled network must not flicker at {now} ms"
        );
    }
}

fn joining(now: u64) -> f32 {
    wifi_alpha(WifiState::Joining, now).expect("joining is drawn")
}

#[test]
fn joining_breathes_between_dim_and_full() {
    let dim = joining(0);
    let full = joining(PULSE_MS / 2);
    assert!(
        dim < 0.45,
        "the dim end of the breath is {dim}, too close to solid to read as working"
    );
    assert!(
        full > 0.99,
        "the bright end of the breath is {full}, the icon never reaches the others"
    );
}

#[test]
fn the_breath_never_leaves_the_visible_range() {
    for ms in 0..(PULSE_MS * 3) {
        let a = joining(ms);
        assert!(
            (0.0..=1.0).contains(&a),
            "alpha {a} at {ms} ms is outside what a texture can hold"
        );
        assert!(a > 0.0, "the icon vanished entirely at {ms} ms");
    }
}

#[test]
fn the_breath_repeats_on_a_fixed_period() {
    for ms in [0, 137, PULSE_MS / 3, PULSE_MS - 1] {
        let a = joining(ms);
        let b = joining(ms + PULSE_MS);
        assert!(
            (a - b).abs() < 0.001,
            "{a} at {ms} ms against {b} one period later"
        );
    }
}

#[test]
fn the_breath_is_a_sweep_rather_than_a_blink() {
    // A hard blink reads as an alarm. Neighbouring frames must barely differ.
    let step = 16;
    for ms in (0..PULSE_MS).step_by(step as usize) {
        let jump = (joining(ms) - joining(ms + step)).abs();
        assert!(
            jump < 0.1,
            "alpha jumped {jump} across one frame at {ms} ms, that is a blink"
        );
    }
}

#[test]
fn the_wifi_icon_is_one_of_the_icons() {
    assert!(
        Icon::ALL.contains(&Icon::Wifi),
        "an icon outside ALL never gets a texture uploaded"
    );
}

mod corner {
    use slot_ui::{
        icon_box, Badge, Draw, FfState, Hud, Icon, LinkBadge, TexId, WifiState, HUD_ICON_PX,
        LINK_HOST_INK, OUT_W, PULSE_MS,
    };

    fn hud() -> Hud {
        let mut h = Hud::new();
        h.set_icons((0..Icon::ALL.len()).map(TexId::from_raw).collect());
        h.set_link_faces(
            LinkBadge::FACES
                .iter()
                .enumerate()
                .map(|(i, _)| TexId::from_raw(900 + i))
                .collect(),
        );
        h
    }

    fn texes(h: &Hud, now: u64) -> Vec<(TexId, f32)> {
        let mut out = Vec::new();
        h.draw(now, &mut out);
        out.iter()
            .filter_map(|d| match *d {
                Draw::Tex { tex, alpha, .. } => Some((tex, alpha)),
                _ => None,
            })
            .collect()
    }

    const WIFI: usize = 12;

    #[test]
    fn a_joining_network_takes_the_corner_over_a_game() {
        let mut h = hud();
        h.set_wifi(WifiState::Joining);
        let mut out = Vec::new();
        h.draw(0, &mut out);
        assert_eq!(out.len(), 1, "the icon came with company: {out:?}");
        let (w, _) = icon_box(HUD_ICON_PX);
        assert!(
            matches!(out[0], Draw::Tex { tex, x, .. }
                if tex == TexId::from_raw(WIFI) && x == OUT_W as f32 - 12.0 - w as f32),
            "the joining icon is not in the badge corner: {out:?}"
        );
    }

    #[test]
    fn a_joined_network_says_nothing_over_a_game() {
        let mut h = hud();
        h.set_wifi(WifiState::Up);
        assert!(
            texes(&h, 0).is_empty(),
            "a settled network must not sit on top of the game"
        );
    }

    #[test]
    fn an_off_network_says_nothing_over_a_game() {
        let mut h = hud();
        h.set_wifi(WifiState::Off);
        assert!(texes(&h, 0).is_empty());
    }

    #[test]
    fn the_corner_icon_breathes_rather_than_holding_still() {
        let mut h = hud();
        h.set_wifi(WifiState::Joining);
        let dim = texes(&h, 0)[0].1;
        let full = texes(&h, PULSE_MS / 2)[0].1;
        assert!(
            full - dim > 0.4,
            "the corner icon went from {dim} to {full}, which nobody will notice"
        );
    }

    #[test]
    fn a_link_session_keeps_the_corner() {
        let mut h = hud();
        h.set_link(LinkBadge::Hosting);
        h.set_wifi(WifiState::Joining);
        let drawn = texes(&h, 0);
        assert_eq!(drawn.len(), 1, "two badges in one corner: {drawn:?}");
        assert_eq!(
            drawn[0].0,
            TexId::from_raw(900),
            "the network took the corner the cable was using"
        );
    }

    #[test]
    fn fast_forward_keeps_the_corner_too() {
        let mut h = hud();
        h.set_ff(FfState::Latched);
        h.set_wifi(WifiState::Joining);
        let drawn = texes(&h, 0);
        assert_eq!(drawn.len(), 1);
        assert_ne!(
            drawn[0].0,
            TexId::from_raw(WIFI),
            "the network pushed aside something the player just asked for"
        );
    }

    #[test]
    fn the_badge_faces_are_what_the_other_corner_users_expect() {
        let _ = (Badge::Link, LINK_HOST_INK);
        assert_eq!(Icon::Wifi.index(), WIFI);
    }
}

mod footer {
    use slot_ui::{draw_footer, Draw, Printed, TexId, OUT_W};

    const WIFI: usize = 77;
    const CLOCK: usize = 88;
    const CLOCK_W: u32 = 60;

    fn footer(wifi: Option<(TexId, f32)>) -> Vec<Draw> {
        let mut out = Vec::new();
        draw_footer(
            None,
            Printed::default(),
            None,
            Printed::new(TexId::from_raw(CLOCK), CLOCK_W),
            wifi,
            &mut out,
        );
        out
    }

    fn tex(out: &[Draw], id: usize) -> Option<(f32, f32)> {
        out.iter().find_map(|d| match *d {
            Draw::Tex { tex, x, alpha, .. } if tex == TexId::from_raw(id) => Some((x, alpha)),
            _ => None,
        })
    }

    #[test]
    fn a_card_with_no_network_adds_nothing_to_the_footer() {
        assert!(
            tex(&footer(None), WIFI).is_none(),
            "an icon appeared for a network that is not there"
        );
    }

    #[test]
    fn the_network_sits_beside_the_clock_rather_than_over_it() {
        let out = footer(Some((TexId::from_raw(WIFI), 1.0)));
        let (wifi_x, _) = tex(&out, WIFI).expect("the icon was not drawn");
        let (clock_x, _) = tex(&out, CLOCK).expect("the clock was not drawn");
        assert!(
            wifi_x < clock_x,
            "the icon at {wifi_x} is not left of the clock at {clock_x}"
        );
        assert!(
            wifi_x > 0.0 && wifi_x < OUT_W as f32,
            "the icon at {wifi_x} is off the panel"
        );
    }

    #[test]
    fn the_footer_icon_wears_the_alpha_it_was_handed() {
        let out = footer(Some((TexId::from_raw(WIFI), 0.42)));
        let (_, alpha) = tex(&out, WIFI).expect("the icon was not drawn");
        assert!(
            (alpha - 0.42).abs() < f32::EPSILON,
            "the footer drew the icon at {alpha}, so the breath never reaches the shelf"
        );
    }
}

mod carried {
    use slot_ui::WifiState;

    #[test]
    fn every_state_survives_the_trip_through_a_number() {
        for (i, state) in WifiState::ALL.iter().enumerate() {
            assert_eq!(state.index(), i, "{state:?} is out of order in ALL");
            assert_eq!(WifiState::from_index(i), *state);
        }
    }

    #[test]
    fn a_number_from_nowhere_reads_as_off() {
        assert_eq!(WifiState::from_index(99), WifiState::Off);
        assert_eq!(WifiState::from_index(usize::MAX), WifiState::Off);
    }
}
