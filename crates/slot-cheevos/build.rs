//! Compiles rcheevos for whatever target cargo is building.
//!
//! The source is not in this repository: `task cheevos:src` fetches the pinned
//! commit into vendor/, which is ignored, exactly as the mGBA and gpSP cores
//! are handled. Compiling here rather than in that script means cargo picks
//! the target, so the host test build and the aarch64 device build each get
//! their own objects with no cross toolchain to arrange. The container
//! `task build:device` uses is already aarch64, so its own cc is the device
//! compiler.

use std::path::{Path, PathBuf};

/// The thirty-one translation units rcheevos needs. Its own Makefile.common
/// lists all but the last, which upstream consumers are expected to add
/// themselves, and cannot be used regardless: it refuses any ARCH that is not
/// x86 or x64.
const SOURCES: &[&str] = &[
    "util/md5.c",
    "util/rc_compat.c",
    "util/rc_util.c",
    "util/rc_version.c",
    "runtime/rc_alloc.c",
    "runtime/rc_condition.c",
    "runtime/rc_condset.c",
    "runtime/rc_consoleinfo.c",
    "runtime/rc_format.c",
    "runtime/rc_lboard.c",
    "runtime/rc_memref.c",
    "runtime/rc_operand.c",
    "runtime/rc_richpresence.c",
    "runtime/rc_runtime.c",
    "runtime/rc_runtime_progress.c",
    "runtime/rc_trigger.c",
    "runtime/rc_value.c",
    "api/rc_api_common.c",
    "api/rc_api_editor.c",
    "api/rc_api_info.c",
    "api/rc_api_runtime.c",
    "api/rc_api_user.c",
    "client/rc_client.c",
    "client/rc_client_external.c",
    "client/rc_client_raintegration.c",
    "hash/rc_cdreader.c",
    "hash/rc_hash.c",
    "hash/rc_hash_disc.c",
    "hash/rc_hash_rom.c",
    "hash/rc_hash_zip.c",
    "libretro/rc_libretro.c",
];

fn main() {
    let crate_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let shim = crate_dir.join("shim");
    let src = crate_dir.join("../../vendor/rcheevos");

    if !src.join("include/rc_client.h").exists() {
        panic!(
            "rcheevos is not in {}: run `task cheevos:src`",
            src.display()
        );
    }

    println!("cargo:rerun-if-changed={}", shim.display());
    println!(
        "cargo:rerun-if-changed={}",
        src.join(".slot-stamp").display()
    );

    let mut build = cc::Build::new();
    build
        .include(src.join("include"))
        .include(&shim)
        .flag("-std=gnu99")
        // Six of the thirty-one fail without this, on an implicit strcasecmp.
        // rcheevos' own Makefile passes it for the same reason.
        .define("_GNU_SOURCE", None)
        // Without this rc_client_begin_identify_and_load_game does not exist.
        // The library still builds; the symbol is simply absent.
        .define("RC_CLIENT_SUPPORTS_HASH", None)
        // Third-party source. Its warnings are not ours to answer, and a
        // compiler we have not tried must not fail the build over one.
        .warnings(false);

    // Large-file support is glibc's spelling of it. rcheevos reads
    // _LARGEFILE64_SOURCE as permission to call fseeko64 and ftello64, which
    // do not exist on macOS, where off_t is already 64 bits and plain fseek is
    // the right call. Defining it everywhere breaks the host build outright.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        build
            .define("_LARGEFILE64_SOURCE", None)
            .define("_FILE_OFFSET_BITS", "64");
    }

    for f in SOURCES {
        let path = src.join("src").join(f);
        assert!(
            path.exists(),
            "rcheevos {} is missing: the pinned commit moved its source tree",
            path.display()
        );
        build.file(path);
    }
    build.file(shim.join("abi.c"));

    build.compile("rcheevos");
    rerun_on_sources(&src.join("src"));
}

/// rcheevos' own headers are not in the file list, so a changed header would
/// otherwise go unnoticed between builds.
fn rerun_on_sources(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            rerun_on_sources(&p);
        } else if matches!(p.extension().and_then(|s| s.to_str()), Some("c" | "h")) {
            println!("cargo:rerun-if-changed={}", p.display());
        }
    }
}
