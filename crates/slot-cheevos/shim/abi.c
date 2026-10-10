/* The shim redeclares two libretro structs for rcheevos, which was compiled
 * against them. Reporting the layout the C compiler actually chose lets a Rust
 * test pin it, so a drift from the libretro ABI fails here rather than showing
 * up as achievements reading plausible-looking wrong bytes.
 */
#include <stddef.h>
#include <libretro.h>

struct slot_descriptor_abi {
	size_t size;
	size_t flags;
	size_t ptr;
	size_t offset;
	size_t start;
	size_t select;
	size_t disconnect;
	size_t len;
	size_t addrspace;
};

void slot_descriptor_abi(struct slot_descriptor_abi *out)
{
	out->size = sizeof(struct retro_memory_descriptor);
	out->flags = offsetof(struct retro_memory_descriptor, flags);
	out->ptr = offsetof(struct retro_memory_descriptor, ptr);
	out->offset = offsetof(struct retro_memory_descriptor, offset);
	out->start = offsetof(struct retro_memory_descriptor, start);
	out->select = offsetof(struct retro_memory_descriptor, select);
	out->disconnect = offsetof(struct retro_memory_descriptor, disconnect);
	out->len = offsetof(struct retro_memory_descriptor, len);
	out->addrspace = offsetof(struct retro_memory_descriptor, addrspace);
}
