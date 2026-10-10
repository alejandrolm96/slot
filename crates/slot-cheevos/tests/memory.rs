//! The flat address space RetroAchievements reads, built from the map mGBA
//! describes.
//!
//! An achievement does not name `0x03000000`. It names `0x000000`, and the
//! three regions RetroAchievements defines for the GBA are laid end to end:
//!
//! ```text
//! 0x000000 - 0x007FFF  ->  0x03000000   32K  IWRAM
//! 0x008000 - 0x047FFF  ->  0x02000000  256K  EWRAM
//! 0x048000 - 0x057FFF  ->  0x0E000000   64K  SRAM
//! ```
//!
//! Getting `0x008000` to land on the first byte of EWRAM is the whole reason
//! this crate links rcheevos instead of translating addresses by hand.

use slot_cheevos::{Console, MappedRegion, Memory};

const IWRAM: usize = 0x8000;
const EWRAM: usize = 0x4_0000;
const SRAM: usize = 0x2_0000;
const ROM: usize = 0x40_0000;
const VRAM: usize = 0x1_8000;
const BIOS: usize = 0x4000;
const SMALL: usize = 0x400;

/// Buffers standing in for the core's own memory, each filled so that a read
/// landing in the wrong one is obvious rather than plausible.
struct Gba {
    iwram: Vec<u8>,
    ewram: Vec<u8>,
    sram: Vec<u8>,
    rom: Vec<u8>,
    bios: Vec<u8>,
    vram: Vec<u8>,
    palette: Vec<u8>,
    oam: Vec<u8>,
    io: Vec<u8>,
}

impl Gba {
    fn new() -> Self {
        let mut g = Gba {
            iwram: vec![0x11; IWRAM],
            ewram: vec![0x22; EWRAM],
            sram: vec![0x33; SRAM],
            rom: vec![0x44; ROM],
            bios: vec![0x55; BIOS],
            vram: vec![0x66; VRAM],
            palette: vec![0x77; SMALL],
            oam: vec![0x88; SMALL],
            io: vec![0x99; SMALL],
        };
        g.iwram[0] = 0xA0;
        g.iwram[IWRAM - 1] = 0xA1;
        g.ewram[0] = 0xB0;
        g.ewram[EWRAM - 1] = 0xB1;
        g.sram[0] = 0xC0;
        g.sram[0xFFFF] = 0xC1;
        g
    }

    /// The eleven descriptors mGBA published on a real RG SP, in the order it
    /// published them. The three ROM waitstate windows are mirrors of one
    /// buffer, which is how the core reports them.
    fn map(&mut self) -> Vec<MappedRegion> {
        let rom = self.rom.as_mut_ptr();
        vec![
            region(
                0x4,
                self.iwram.as_mut_ptr(),
                0x0300_0000,
                0xFF00_0000,
                IWRAM,
            ),
            region(
                0x4,
                self.ewram.as_mut_ptr(),
                0x0200_0000,
                0xFF00_0000,
                EWRAM,
            ),
            region(0x0, self.sram.as_mut_ptr(), 0x0E00_0000, 0, SRAM),
            region(0x1, rom, 0x0800_0000, 0, ROM),
            region(0x1, rom, 0x0A00_0000, 0, ROM),
            region(0x1, rom, 0x0C00_0000, 0, ROM),
            region(0x1, self.bios.as_mut_ptr(), 0x0000_0000, 0, BIOS),
            region(0x0, self.vram.as_mut_ptr(), 0x0600_0000, 0xFF00_0000, VRAM),
            region(
                0x0,
                self.palette.as_mut_ptr(),
                0x0500_0000,
                0xFF00_0000,
                SMALL,
            ),
            region(0x0, self.oam.as_mut_ptr(), 0x0700_0000, 0xFF00_0000, SMALL),
            region(0x0, self.io.as_mut_ptr(), 0x0400_0000, 0, SMALL),
        ]
    }
}

fn region(flags: u64, ptr: *mut u8, start: usize, select: usize, len: usize) -> MappedRegion {
    MappedRegion {
        flags,
        ptr,
        offset: 0,
        start,
        select,
        disconnect: 0,
        len,
    }
}

/// SAFETY: `gba` outlives the returned Memory in every caller below.
unsafe fn mapped(gba: &mut Gba) -> Memory {
    let map = gba.map();
    Memory::new(&map, Console::GameBoyAdvance).expect("rcheevos refused the map mGBA publishes")
}

fn byte(m: &Memory, address: u32) -> Option<u8> {
    let mut b = [0u8; 1];
    match m.read(address, &mut b) {
        1 => Some(b[0]),
        _ => None,
    }
}

#[test]
fn the_three_regions_retroachievements_defines_for_the_gba_are_laid_end_to_end() {
    let mut gba = Gba::new();
    let m = unsafe { mapped(&mut gba) };
    assert_eq!(
        m.len(),
        0x5_8000,
        "the flat space is 32K + 256K + 64K; a different total means a region was \
         missed, doubled, or sized off the cartridge"
    );
}

#[test]
fn the_first_address_reads_the_first_byte_of_internal_work_ram() {
    let mut gba = Gba::new();
    let m = unsafe { mapped(&mut gba) };
    assert_eq!(byte(&m, 0x00_0000), Some(0xA0));
    assert_eq!(byte(&m, 0x00_7FFF), Some(0xA1), "the last byte of IWRAM");
}

#[test]
fn the_boundary_at_0x8000_crosses_into_external_work_ram() {
    // The one that matters. IWRAM is 32K, so 0x008000 is the first byte of
    // EWRAM at 0x02000000, a different buffer at a lower hardware address.
    // Every achievement that reads the main work RAM depends on this.
    let mut gba = Gba::new();
    let m = unsafe { mapped(&mut gba) };
    assert_eq!(byte(&m, 0x00_7FFF), Some(0xA1), "still IWRAM");
    assert_eq!(
        byte(&m, 0x00_8000),
        Some(0xB0),
        "0x8000 did not cross into EWRAM, so achievements would read the wrong memory"
    );
    assert_eq!(byte(&m, 0x04_7FFF), Some(0xB1), "the last byte of EWRAM");
}

#[test]
fn save_ram_follows_external_work_ram_and_is_clamped_to_the_64k_retroachievements_defines() {
    // mGBA offers 128K at 0x0E000000; RetroAchievements defines 64K of Save
    // RAM. The smaller window is the one that counts, or every address past it
    // would shift.
    let mut gba = Gba::new();
    let m = unsafe { mapped(&mut gba) };
    assert_eq!(byte(&m, 0x04_8000), Some(0xC0), "the first byte of SRAM");
    assert_eq!(byte(&m, 0x05_7FFF), Some(0xC1), "the last byte of SRAM");
    assert_eq!(
        byte(&m, 0x05_8000),
        None,
        "reading past the flat space answered something"
    );
}

#[test]
fn a_run_of_bytes_across_the_seam_is_stitched_from_both_regions_in_order() {
    // The flat space is contiguous even though the hardware is not, so a
    // thirty-two bit read at 0x7FFE legitimately takes two bytes from the tail
    // of IWRAM and two from the head of EWRAM, which sits at a lower hardware
    // address in a different buffer. Refusing to stitch would break every
    // achievement that reads a word near the boundary.
    let mut gba = Gba::new();
    let m = unsafe { mapped(&mut gba) };
    let mut buf = [0u8; 4];
    assert_eq!(m.read(0x00_7FFE, &mut buf), 4, "the seam was not crossed");
    assert_eq!(
        buf,
        [0x11, 0xA1, 0xB0, 0x22],
        "the four bytes are IWRAM filler, the last byte of IWRAM, the first \
         byte of EWRAM, then EWRAM filler; anything else stitched the wrong \
         buffers or stitched them backwards"
    );
}

#[test]
fn a_read_that_starts_past_the_end_is_refused_rather_than_wrapped() {
    let mut gba = Gba::new();
    let m = unsafe { mapped(&mut gba) };
    let mut buf = [0u8; 4];
    assert_eq!(
        m.read(0x06_0000, &mut buf),
        0,
        "an address beyond the flat space was served from somewhere"
    );
}

#[test]
fn a_core_that_described_nothing_maps_nothing() {
    // SAFETY: an empty map carries no pointers to outlive anything.
    let built = unsafe { Memory::new(&[], Console::GameBoyAdvance) };
    assert!(
        built.is_none(),
        "an empty map produced a memory to read, which would be read as zeros \
         and evaluated as game state"
    );
}

#[test]
fn the_shim_header_agrees_with_the_libretro_abi() {
    // The shim redeclares struct retro_memory_descriptor for rcheevos. If its
    // layout drifts from the one mGBA was compiled against, every field
    // rcheevos reads is off and the symptom is wrong bytes, not a crash.
    let abi = slot_cheevos::descriptor_abi();
    assert_eq!(abi.size, 64, "struct retro_memory_descriptor is 64 bytes");
    assert_eq!(abi.flags, 0);
    assert_eq!(abi.ptr, 8);
    assert_eq!(abi.offset, 16);
    assert_eq!(abi.start, 24);
    assert_eq!(abi.select, 32);
    assert_eq!(abi.disconnect, 40);
    assert_eq!(abi.len, 48);
    assert_eq!(abi.addrspace, 56);
}
