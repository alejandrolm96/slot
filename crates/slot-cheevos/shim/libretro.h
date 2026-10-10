/* rcheevos' rc_libretro.h includes <libretro.h> and says so itself: "this file
 * comes from the libretro repository, which is not an explicit submodule. the
 * integration must set up paths appropriately to find it."
 *
 * Sweeping src/libretro for libretro symbols turns up exactly two structs and
 * three constants, so this is all of libretro.h that rcheevos needs. Vendoring
 * the real four-thousand-line header would be four thousand lines of nothing.
 *
 * These definitions are the libretro ABI and must match, field for field, the
 * ones slot-retro mirrors in Rust in crates/slot-retro/src/ffi.rs. A test in
 * this crate holds the two to the same size and offsets.
 */
#ifndef LIBRETRO_H__
#define LIBRETRO_H__

#include <stddef.h>
#include <stdint.h>

#define RETRO_MEMORY_SAVE_RAM 0
#define RETRO_MEMORY_RTC 1
#define RETRO_MEMORY_SYSTEM_RAM 2
#define RETRO_MEMORY_VIDEO_RAM 3

struct retro_memory_descriptor {
   uint64_t flags;
   void *ptr;
   size_t offset;
   size_t start;
   size_t select;
   size_t disconnect;
   size_t len;
   const char *addrspace;
};

struct retro_memory_map {
   const struct retro_memory_descriptor *descriptors;
   unsigned num_descriptors;
};

#endif
