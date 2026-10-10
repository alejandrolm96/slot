use std::fmt;
use std::path::Path;

use crate::link::Link;
use crate::rumble::Rumble;

pub const GBA_W: u32 = 240;
pub const GBA_H: u32 = 160;

#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct ButtonMask(pub u16);

impl ButtonMask {
    pub const B: u16 = 1 << 0;
    pub const Y: u16 = 1 << 1;
    pub const SELECT: u16 = 1 << 2;
    pub const START: u16 = 1 << 3;
    pub const UP: u16 = 1 << 4;
    pub const DOWN: u16 = 1 << 5;
    pub const LEFT: u16 = 1 << 6;
    pub const RIGHT: u16 = 1 << 7;
    pub const A: u16 = 1 << 8;
    pub const X: u16 = 1 << 9;
    pub const L: u16 = 1 << 10;
    pub const R: u16 = 1 << 11;

    pub fn without_turbo(self) -> ButtonMask {
        ButtonMask(self.0 & !(Self::X | Self::Y))
    }

    pub fn turbo(self, frame: u32) -> ButtonMask {
        let mut mask = self.0 & !(Self::X | Self::Y);
        if (frame / 3).is_multiple_of(2) {
            if self.0 & Self::X != 0 {
                mask |= Self::A;
            }
            if self.0 & Self::Y != 0 {
                mask |= Self::B;
            }
        }
        ButtonMask(mask)
    }
}

/// One range of the emulated machine's address space, as the core described
/// it. The pointer the core gave alongside this stays inside the crate: it
/// points into the core's own memory and is only good while that core lives.
///
/// Decoding an address needs all of these, so none is dropped on the way
/// through: `start` and `len` place the range, `select` says which bits of an
/// address must match it, `disconnect` says which are not wired at all, and
/// `offset` shifts into the core's buffer.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct MemoryRegion {
    pub flags: u64,
    pub offset: usize,
    pub start: usize,
    pub select: usize,
    pub disconnect: usize,
    pub len: usize,
}

#[derive(Copy, Clone, Debug)]
pub struct AvInfo {
    pub fps: f64,
    pub sample_rate: f64,
}

#[derive(Debug)]
pub enum CoreError {
    Io(std::io::Error),
    Load(String),
    Unsupported(String),
    State(String),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::Io(e) => write!(f, "io: {e}"),
            CoreError::Load(m) => write!(f, "load: {m}"),
            CoreError::Unsupported(m) => write!(f, "unsupported: {m}"),
            CoreError::State(m) => write!(f, "state: {m}"),
        }
    }
}

impl std::error::Error for CoreError {}

impl From<std::io::Error> for CoreError {
    fn from(e: std::io::Error) -> Self {
        CoreError::Io(e)
    }
}

pub trait RetroCore: Send {
    fn load(&mut self, rom: &Path) -> Result<(), CoreError>;
    fn run_frame(&mut self, input: ButtonMask);
    fn run_frame_linked(&mut self, p1: ButtonMask, _p2: ButtonMask) {
        self.run_frame(p1);
    }
    fn set_option(&mut self, _key: &str, _value: &str) {}
    fn set_frame_skip(&mut self, _skip: bool) {}
    fn video_xrgb8888(&self) -> &[u8];
    fn take_audio(&mut self) -> Vec<i16>;
    fn serialize(&mut self) -> Result<Vec<u8>, CoreError>;
    fn unserialize(&mut self, data: &[u8]) -> Result<(), CoreError>;
    fn save_ram(&self) -> Option<Vec<u8>>;
    fn load_save_ram(&mut self, data: &[u8]) -> Result<(), CoreError>;
    fn av_info(&self) -> AvInfo;
    fn rumble(&self) -> Rumble {
        Rumble::default()
    }
    fn net(&self) -> Link {
        Link::default()
    }
    /// What the core said about its address space, empty when it said
    /// nothing. Achievements are decided against this memory, so a core that
    /// describes none can carry none.
    ///
    /// A core may not have described anything yet when it is merely loaded.
    /// mGBA defers its own setup to the first `retro_run`, so ask after a
    /// frame has gone through, never straight after `load`.
    fn memory_regions(&self) -> Vec<MemoryRegion> {
        Vec::new()
    }

    /// Whether the core declared that it supports achievements.
    fn supports_achievements(&self) -> bool {
        false
    }

    fn start_link(&mut self, _client_id: u16) {}
    fn pump_link(&mut self) {}
    fn stop_link(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Recorder(Vec<ButtonMask>);

    impl RetroCore for Recorder {
        fn load(&mut self, _rom: &Path) -> Result<(), CoreError> {
            Ok(())
        }
        fn run_frame(&mut self, input: ButtonMask) {
            self.0.push(input);
        }
        fn video_xrgb8888(&self) -> &[u8] {
            &[]
        }
        fn take_audio(&mut self) -> Vec<i16> {
            Vec::new()
        }
        fn serialize(&mut self) -> Result<Vec<u8>, CoreError> {
            Ok(Vec::new())
        }
        fn unserialize(&mut self, _data: &[u8]) -> Result<(), CoreError> {
            Ok(())
        }
        fn save_ram(&self) -> Option<Vec<u8>> {
            None
        }
        fn load_save_ram(&mut self, _data: &[u8]) -> Result<(), CoreError> {
            Ok(())
        }
        fn av_info(&self) -> AvInfo {
            AvInfo {
                fps: 60.0,
                sample_rate: 48_000.0,
            }
        }
    }

    #[test]
    fn a_core_without_link_mode_runs_player_1_alone() {
        let mut core = Recorder::default();
        core.run_frame_linked(ButtonMask(ButtonMask::A), ButtonMask(ButtonMask::B));
        assert_eq!(core.0, vec![ButtonMask(ButtonMask::A)]);
    }
}
