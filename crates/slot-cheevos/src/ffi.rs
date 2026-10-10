//! Hand-written declarations for the rcheevos entry points this crate uses,
//! mirroring rc_libretro.h and the shim header beside it. slot-retro declares
//! the libretro ABI the same way; a generator would be a new tool and a new
//! dependency for a surface this small.

use std::ffi::c_void;
use std::os::raw::{c_int, c_uint};

/// Mirrors `struct retro_memory_descriptor`, which rcheevos reads through the
/// shim header. The field order is the ABI.
#[repr(C)]
pub struct MemoryDescriptor {
    pub flags: u64,
    pub ptr: *mut c_void,
    pub offset: usize,
    pub start: usize,
    pub select: usize,
    pub disconnect: usize,
    pub len: usize,
    pub addrspace: *const std::os::raw::c_char,
}

/// Mirrors `struct retro_memory_map`.
#[repr(C)]
pub struct MemoryMap {
    pub descriptors: *const MemoryDescriptor,
    pub num_descriptors: c_uint,
}

/// `RC_LIBRETRO_MAX_MEMORY_REGIONS`. rcheevos writes this many slots at most,
/// so the arrays below cannot be shortened.
pub const MAX_REGIONS: usize = 32;

/// Mirrors `rc_libretro_memory_regions_t`. rcheevos fills it in; nothing here
/// writes to it.
#[repr(C)]
pub struct MemoryRegions {
    pub data: [*mut u8; MAX_REGIONS],
    pub size: [usize; MAX_REGIONS],
    pub total_size: usize,
    pub count: c_uint,
}

impl MemoryRegions {
    pub fn empty() -> Self {
        MemoryRegions {
            data: [std::ptr::null_mut(); MAX_REGIONS],
            size: [0; MAX_REGIONS],
            total_size: 0,
            count: 0,
        }
    }
}

/// Mirrors `rc_libretro_core_memory_info_t`.
#[repr(C)]
pub struct CoreMemoryInfo {
    pub data: *mut u8,
    pub size: usize,
}

pub type GetCoreMemoryInfo = unsafe extern "C" fn(id: c_uint, info: *mut CoreMemoryInfo);

/// What the C compiler made of the shim's `struct retro_memory_descriptor`.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct DescriptorAbi {
    pub size: usize,
    pub flags: usize,
    pub ptr: usize,
    pub offset: usize,
    pub start: usize,
    pub select: usize,
    pub disconnect: usize,
    pub len: usize,
    pub addrspace: usize,
}

extern "C" {
    pub fn rc_libretro_memory_init(
        regions: *mut MemoryRegions,
        mmap: *const MemoryMap,
        get_core_memory_info: GetCoreMemoryInfo,
        console_id: c_uint,
    ) -> c_int;

    pub fn rc_libretro_memory_destroy(regions: *mut MemoryRegions);

    pub fn rc_libretro_memory_read(
        regions: *const MemoryRegions,
        address: c_uint,
        buffer: *mut u8,
        num_bytes: c_uint,
    ) -> c_uint;

    pub fn slot_descriptor_abi(out: *mut DescriptorAbi);
}
