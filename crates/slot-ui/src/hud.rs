use slot_gfx::{Draw, TexId, OUT_W};
use slot_store::{BLUE_LIGHT_MAX, BRIGHTNESS_MAX, VOLUME_MAX};

use crate::icon::icon_box;
use crate::toast::toast_rect;
use crate::{Badge, Icon, Toast};

pub type Millis = u64;

pub const HUD_MS: Millis = 1500;
const FADE_MS: Millis = 250;

pub const PLATE_H: f32 = 40.0;
pub(crate) const PLATE: [f32; 4] = [0.0, 0.0, 0.0, 0.72];

pub const HUD_ICON_PX: f32 = 24.0;
pub const HUD_INK: [u8; 3] = [0xf5, 0xf2, 0xef];
const ICON_GAP: f32 = 10.0;

const BADGE_MARGIN: f32 = 12.0;

pub fn badge_at(w: f32, h: f32) -> (f32, f32) {
    (OUT_W as f32 - BADGE_MARGIN - w, (PLATE_H - h) / 2.0)
}

const BAR_W: f32 = 320.0;
const BAR_H: f32 = 6.0;
const BAR_Y: f32 = (PLATE_H - BAR_H) / 2.0;
pub(crate) const TRACK: [f32; 4] = [1.0, 1.0, 1.0, 0.18];
pub(crate) const FILL: [f32; 4] = [
    HUD_INK[0] as f32 / 255.0,
    HUD_INK[1] as f32 / 255.0,
    HUD_INK[2] as f32 / 255.0,
    1.0,
];

/// How much of the home network the card has: nothing, a join in progress, or
/// an address it can actually use.
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub enum WifiState {
    #[default]
    Off,
    Joining,
    Up,
}

impl WifiState {
    pub const ALL: [WifiState; 3] = [WifiState::Off, WifiState::Joining, WifiState::Up];

    pub fn index(self) -> usize {
        self as usize
    }

    /// The worker keeps this in an atomic, so it travels as a number. An
    /// unknown one means off rather than a panic: a torn read must not take
    /// the frontend down.
    pub fn from_index(i: usize) -> WifiState {
        Self::ALL.get(i).copied().unwrap_or(WifiState::Off)
    }
}

/// One breath of the joining icon. Long enough to read as work rather than as
/// an alarm, short enough to cycle a few times over the seven seconds a join
/// takes.
pub const PULSE_MS: Millis = 1200;

const PULSE_DIM: f32 = 0.3;

/// The alpha to draw the Wi-Fi icon at, or None when there is nothing to draw.
/// Joining sweeps between dim and full rather than blinking: a hard edge reads
/// as something being wrong, and nothing is wrong, it is just not ready.
pub fn wifi_alpha(state: WifiState, now: Millis) -> Option<f32> {
    match state {
        WifiState::Off => None,
        WifiState::Up => Some(1.0),
        WifiState::Joining => {
            let turn = (now % PULSE_MS) as f32 / PULSE_MS as f32;
            let sweep = 0.5 - 0.5 * (turn * std::f32::consts::TAU).cos();
            Some(PULSE_DIM + (1.0 - PULSE_DIM) * sweep)
        }
    }
}

#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub enum HudKind {
    #[default]
    Brightness,
    BlueLight,
    Volume,
    Rewind,
}

impl HudKind {
    pub fn icon(self, value: u8, muted: bool, headphones: bool) -> Icon {
        match self {
            HudKind::Brightness => Icon::Brightness,
            HudKind::BlueLight => Icon::BlueLight,
            HudKind::Volume if headphones && muted => Icon::HeadphonesMuted,
            HudKind::Volume if headphones => Icon::Headphones,
            HudKind::Volume if muted => Icon::VolumeMuted,
            HudKind::Volume if value == 0 => Icon::VolumeZero,
            HudKind::Volume => Icon::Volume,
            HudKind::Rewind => Icon::Rewind,
        }
    }

    fn max(self) -> u8 {
        match self {
            HudKind::Brightness => BRIGHTNESS_MAX,
            HudKind::BlueLight => BLUE_LIGHT_MAX,
            HudKind::Volume => VOLUME_MAX,
            HudKind::Rewind => 100,
        }
    }
}

#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub enum FfState {
    #[default]
    Off,
    Held,
    Latched,
}

pub fn ff_badge(state: FfState) -> Option<Icon> {
    match state {
        FfState::Off => None,
        FfState::Held => Some(Icon::FastForward),
        FfState::Latched => Some(Icon::FastForwardLatched),
    }
}

pub const LINK_HOST_INK: [u8; 3] = [0x8a, 0x74, 0xcf];
pub const LINK_JOIN_INK: [u8; 3] = [0xb2, 0xb2, 0xb8];

#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub enum LinkBadge {
    #[default]
    Off,
    Hosting,
    Joined,
    HostingLost,
    JoinedLost,
}

impl LinkBadge {
    pub const FACES: [LinkBadge; 4] = [
        LinkBadge::Hosting,
        LinkBadge::Joined,
        LinkBadge::HostingLost,
        LinkBadge::JoinedLost,
    ];

    pub fn face_index(self) -> Option<usize> {
        LinkBadge::FACES.iter().position(|b| *b == self)
    }

    pub fn badge(self) -> Option<Badge> {
        match self {
            LinkBadge::Off => None,
            LinkBadge::Hosting | LinkBadge::Joined => Some(Badge::Link),
            LinkBadge::HostingLost | LinkBadge::JoinedLost => Some(Badge::LinkBroken),
        }
    }

    pub fn colour(self) -> Option<[u8; 3]> {
        match self {
            LinkBadge::Off => None,
            LinkBadge::Hosting | LinkBadge::HostingLost => Some(LINK_HOST_INK),
            LinkBadge::Joined | LinkBadge::JoinedLost => Some(LINK_JOIN_INK),
        }
    }
}

#[derive(Default)]
pub struct Hud {
    pub kind: HudKind,
    pub value: u8,
    pub shown_at: Option<Millis>,
    held: bool,
    muted: bool,
    headphones: bool,
    ff: FfState,
    said: Option<(Toast, Millis)>,
    icons: Vec<TexId>,
    toasts: Vec<TexId>,
    link: LinkBadge,
    link_faces: Vec<TexId>,
    wifi: WifiState,
}

impl Hud {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_wifi(&mut self, state: WifiState) {
        self.wifi = state;
    }

    pub fn set_icons(&mut self, icons: Vec<TexId>) {
        self.icons = icons;
    }

    /// The uploaded face for one icon, for callers that draw outside the plate.
    pub fn face(&self, icon: Icon) -> Option<TexId> {
        self.icons.get(icon.index()).copied()
    }

    pub fn set_toasts(&mut self, toasts: Vec<TexId>) {
        self.toasts = toasts;
    }

    pub fn toast(&mut self, toast: Toast, now: Millis) {
        self.said = Some((toast, now));
    }

    pub fn toast_visible(&self, now: Millis) -> bool {
        self.toast_alpha(now) > 0.0
    }

    pub fn said(&self, now: Millis) -> Option<Toast> {
        self.toast_visible(now)
            .then(|| self.said.map(|(t, _)| t))
            .flatten()
    }

    pub fn show(&mut self, kind: HudKind, value: u8, muted: bool, now: Millis) {
        self.kind = kind;
        self.value = value;
        self.muted = muted;
        self.shown_at = Some(now);
        self.held = kind == HudKind::Rewind;
    }

    pub fn release_rewind(&mut self) {
        if self.kind == HudKind::Rewind {
            self.shown_at = None;
        }
        self.held = false;
    }

    pub fn glyph(&self) -> Icon {
        self.kind.icon(self.value, self.muted, self.headphones)
    }

    pub fn set_headphones(&mut self, on: bool) {
        self.headphones = on;
    }

    pub fn badge(&self) -> Option<Icon> {
        ff_badge(self.ff)
    }

    pub fn set_ff(&mut self, ff: FfState) {
        self.ff = ff;
    }

    pub fn set_link(&mut self, badge: LinkBadge) {
        self.link = badge;
    }

    pub fn link(&self) -> LinkBadge {
        self.link
    }

    pub fn set_link_faces(&mut self, faces: Vec<TexId>) {
        self.link_faces = faces;
    }

    pub fn visible(&self, now: Millis) -> bool {
        self.alpha(now) > 0.0
    }

    pub fn draw(&self, now: Millis, out: &mut Vec<Draw>) {
        let alpha = self.alpha(now);
        let toast = self.toast_alpha(now);
        if alpha > 0.0 || toast > 0.0 {
            out.push(Draw::Rect {
                x: 0.0,
                y: 0.0,
                w: OUT_W as f32,
                h: PLATE_H,
                colour: faded(PLATE, alpha.max(toast)),
            });
        }
        if toast > 0.0 {
            self.draw_toast(now, out);
        } else if alpha > 0.0 {
            self.draw_bar(alpha, out);
        }
        let link = self
            .link
            .face_index()
            .and_then(|i| self.link_faces.get(i).copied());
        let ff = ff_badge(self.ff).and_then(|i| self.icons.get(i.index()).copied());
        if let Some(tex) = link.or(ff) {
            self.place_badge(tex, 1.0, out);
            return;
        }
        // Only a join is worth saying over a game. A settled network is the
        // ordinary state and an off one was asked for, so neither earns pixels
        // in front of what the player is doing.
        if self.wifi != WifiState::Joining {
            return;
        }
        if let (Some(tex), Some(alpha)) = (
            self.icons.get(Icon::Wifi.index()).copied(),
            wifi_alpha(self.wifi, now),
        ) {
            self.place_badge(tex, alpha, out);
        }
    }

    fn draw_bar(&self, alpha: f32, out: &mut Vec<Draw>) {
        let x = (OUT_W as f32 - BAR_W) / 2.0;
        if let Some(tex) = self.icon() {
            let (w, h) = icon_box(HUD_ICON_PX);
            out.push(Draw::Tex {
                x: x - ICON_GAP - w as f32,
                y: (PLATE_H - h as f32) / 2.0,
                w: w as f32,
                h: h as f32,
                tex,
                alpha,
            });
        }
        out.push(Draw::Rect {
            x,
            y: BAR_Y,
            w: BAR_W,
            h: BAR_H,
            colour: faded(TRACK, alpha),
        });
        out.push(Draw::Rect {
            x,
            y: BAR_Y,
            w: BAR_W * self.fraction(),
            h: BAR_H,
            colour: faded(FILL, alpha),
        });
    }

    fn place_badge(&self, tex: TexId, alpha: f32, out: &mut Vec<Draw>) {
        let (w, h) = icon_box(HUD_ICON_PX);
        let (w, h) = (w as f32, h as f32);
        let (x, y) = badge_at(w, h);
        out.push(Draw::Tex {
            x,
            y,
            w,
            h,
            tex,
            alpha,
        });
    }

    fn draw_toast(&self, now: Millis, out: &mut Vec<Draw>) {
        let alpha = self.toast_alpha(now);
        if alpha <= 0.0 {
            return;
        }
        let Some(tex) = self
            .said
            .and_then(|(t, _)| self.toasts.get(t.index()))
            .copied()
        else {
            return;
        };
        let (x, y, w, h) = toast_rect();
        out.push(Draw::Tex {
            x,
            y,
            w,
            h,
            tex,
            alpha,
        });
    }

    fn icon(&self) -> Option<TexId> {
        self.icons.get(self.glyph().index()).copied()
    }

    fn fraction(&self) -> f32 {
        let max = self.kind.max();
        if max == 0 {
            return 0.0;
        }
        self.value.min(max) as f32 / max as f32
    }

    fn alpha(&self, now: Millis) -> f32 {
        let Some(shown) = self.shown_at else {
            return 0.0;
        };
        if self.held {
            return 1.0;
        }
        fade(shown, now)
    }

    fn toast_alpha(&self, now: Millis) -> f32 {
        match self.said {
            Some((_, shown)) => fade(shown, now),
            None => 0.0,
        }
    }
}

fn fade(shown: Millis, now: Millis) -> f32 {
    let age = now.saturating_sub(shown);
    if age >= HUD_MS {
        return 0.0;
    }
    ((HUD_MS - age) as f32 / FADE_MS as f32).min(1.0)
}

fn faded(colour: [f32; 4], alpha: f32) -> [f32; 4] {
    [colour[0], colour[1], colour[2], colour[3] * alpha]
}
