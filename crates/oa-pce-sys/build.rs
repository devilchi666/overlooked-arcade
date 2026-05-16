// oa-pce-sys build script.
//
// Compiles the full Beetle PCE Fast (Mednafen PCE Fast) core via the libretro
// surface. The libretro.cpp file in the upstream tree IS the core engine — it
// owns the PCE_* globals and lifecycle — so we keep it intact and call its
// retro_init / retro_load_game / retro_run from our C++ shim layer.
//
// Default flags here match Beetle's canonical Makefile defaults:
//   - NEED_CD, NEED_TREMOR, NEED_BLIP, NEED_CRC32
//   - HAVE_CHD (libchdr + lzma + zstd + zlib all vendored under vendor/deps)
//   - WANT_PCE_FAST_EMU, WANT_STEREO_SOUND
//   - USE_CHEATS, FRONTEND_SUPPORTS_RGB565
//
// Integration shims (each is one line of pain saved future-us):
//   - INLINE = __inline (Mednafen headers expect this pre-defined)
//   - _CRT_SECURE_NO_WARNINGS (MSVC complains about strncpy/sprintf etc.)
//   - HAVE_STDINT_H + HAVE_INTTYPES_H (libretro headers gate things on these)

use std::path::{Path, PathBuf};

fn main() {
    let vendor = PathBuf::from("vendor");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=vendor");

    // ---- explicit file lists for the core ----
    let mednafen_root_cxx: &[&str] = &[
        "mednafen/general.cpp",
        "mednafen/FileStream.cpp",
        "mednafen/MemoryStream.cpp",
        "mednafen/Stream.cpp",
        "mednafen/mempatcher.cpp",
        "mednafen/okiadpcm.cpp",
    ];
    let mednafen_root_c: &[&str] = &[
        "mednafen/file.c",
        "mednafen/settings.c",
        "mednafen/state.c",
        "mednafen/mednafen-endian.c",
    ];
    let pce_fast_cxx: &[&str] = &[
        "mednafen/pce_fast/pcecd.cpp",
        "mednafen/pce_fast/pcecd_drive.cpp",
        "mednafen/pce_fast/psg.cpp",
        "mednafen/hw_misc/arcade_card/arcade_card.cpp",
    ];
    let pce_fast_c: &[&str] = &[
        "mednafen/pce_fast/huc6280.c",
        "mednafen/pce_fast/input.c",
        "mednafen/pce_fast/vdc.c",
    ];
    let cdrom_cxx: &[&str] = &[
        "mednafen/cdrom/CDAccess.cpp",
        "mednafen/cdrom/CDAccess_Image.cpp",
        "mednafen/cdrom/CDAccess_CCD.cpp",
        "mednafen/cdrom/CDAccess_CHD.cpp",
        "mednafen/cdrom/CDAFReader.cpp",
        "mednafen/cdrom/CDAFReader_Vorbis.cpp",
        "mednafen/cdrom/cdromif.cpp",
        "mednafen/cdrom/CDUtility.cpp",
        "mednafen/cdrom/lec.cpp",
        "mednafen/cdrom/galois.cpp",
        "mednafen/cdrom/recover-raw.cpp",
        "mednafen/cdrom/l-ec.cpp",
        "mednafen/cdrom/edc_crc32.cpp",
    ];
    let sound_c: &[&str] = &["mednafen/sound/Blip_Buffer.c"];
    let libretro_root: &[&str] = &["libretro.cpp"];

    // libretro-common pieces (Makefile.common's STATIC_LINKING != 1 list)
    let libretro_common_c: &[&str] = &[
        "libretro-common/streams/file_stream.c",
        "libretro-common/streams/file_stream_transforms.c",
        "libretro-common/file/file_path.c",
        "libretro-common/file/retro_dirent.c",
        "libretro-common/lists/string_list.c",
        "libretro-common/lists/dir_list.c",
        "libretro-common/compat/compat_strl.c",
        "libretro-common/compat/compat_snprintf.c",
        "libretro-common/compat/compat_posix_string.c",
        "libretro-common/compat/compat_strcasestr.c",
        "libretro-common/compat/fopen_utf8.c",
        "libretro-common/encodings/encoding_utf.c",
        "libretro-common/encodings/encoding_crc32.c",
        "libretro-common/memmap/memalign.c",
        "libretro-common/string/stdstring.c",
        "libretro-common/time/rtime.c",
        "libretro-common/vfs/vfs_implementation.c",
    ];

    // libchdr + lzma + zstd-decompress (vendored under deps/, used for CHD CD images)
    let libchdr_c: &[&str] = &[
        "deps/lzma-19.00/src/Alloc.c",
        "deps/lzma-19.00/src/Bra86.c",
        "deps/lzma-19.00/src/BraIA64.c",
        "deps/lzma-19.00/src/CpuArch.c",
        "deps/lzma-19.00/src/Delta.c",
        "deps/lzma-19.00/src/LzFind.c",
        "deps/lzma-19.00/src/Lzma86Dec.c",
        "deps/lzma-19.00/src/LzmaDec.c",
        "deps/lzma-19.00/src/LzmaEnc.c",
        "deps/libchdr/src/libchdr_bitstream.c",
        "deps/libchdr/src/libchdr_cdrom.c",
        "deps/libchdr/src/libchdr_chd.c",
        "deps/libchdr/src/libchdr_flac.c",
        "deps/libchdr/src/libchdr_huffman.c",
        "deps/zstd/lib/common/entropy_common.c",
        "deps/zstd/lib/common/error_private.c",
        "deps/zstd/lib/common/fse_decompress.c",
        "deps/zstd/lib/common/zstd_common.c",
        "deps/zstd/lib/common/xxhash.c",
        "deps/zstd/lib/decompress/huf_decompress.c",
        "deps/zstd/lib/decompress/zstd_ddict.c",
        "deps/zstd/lib/decompress/zstd_decompress.c",
        "deps/zstd/lib/decompress/zstd_decompress_block.c",
    ];
    let zlib_c: &[&str] = &[
        "deps/zlib-1.2.11/adler32.c",
        "deps/zlib-1.2.11/crc32.c",
        "deps/zlib-1.2.11/inffast.c",
        "deps/zlib-1.2.11/inflate.c",
        "deps/zlib-1.2.11/inftrees.c",
        "deps/zlib-1.2.11/zutil.c",
    ];

    // tremor — directory glob, excluding the example file (per Makefile.common).
    let tremor_dir = vendor.join("mednafen/tremor");
    let tremor_c: Vec<PathBuf> = read_dir_files(&tremor_dir, "c")
        .into_iter()
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            !name.ends_with("ivorbisfile_example.c")
        })
        .collect();

    // ---- common build configuration ----
    let common_includes = [
        "",                              // CORE_DIR
        "mednafen",
        "mednafen/include",
        "mednafen/hw_sound",
        "mednafen/hw_cpu",
        "mednafen/hw_misc",
        "libretro-common/include",
        "deps/lzma-19.00/include",
        "deps/libchdr/include",
        "deps/zstd/lib",
        "deps/zlib-1.2.11",
    ];

    let common_defines: &[(&str, Option<&str>)] = &[
        ("INLINE", Some("__inline")),
        ("_CRT_SECURE_NO_WARNINGS", None),
        ("_CRT_SECURE_NO_DEPRECATE", None),
        ("HAVE_STDINT_H", None),
        ("HAVE_INTTYPES_H", None),
        ("STDC_HEADERS", None),
        ("__STDC_LIMIT_MACROS", None),
        ("__STDC_CONSTANT_MACROS", None),
        // Beetle's Makefile pins this to 931 (Mednafen 0.9.31-ish).
        ("MEDNAFEN_VERSION_NUMERIC", Some("931")),
        ("MEDNAFEN_VERSION", Some("\"0.9.26\"")),
        ("_LOW_ACCURACY_", None),
        ("WANT_PCE_FAST_EMU", None),
        ("WANT_STEREO_SOUND", None),
        ("FRONTEND_SUPPORTS_RGB565", None),
        ("NEED_CD", None),
        ("NEED_TREMOR", None),
        ("NEED_BLIP", None),
        ("NEED_CRC32", None),
        ("NEED_DEINTERLACER", None),
        ("USE_CHEATS", None),
        ("HAVE_CHD", None),
        ("_7ZIP_ST", None),
        ("ZSTD_DISABLE_ASM", None),
        ("__LIBRETRO__", None),
    ];

    // ---- C build ----
    let mut c_build = cc::Build::new();
    apply_includes(&mut c_build, &vendor, &common_includes);
    apply_defines(&mut c_build, common_defines);
    c_build.warnings(false);

    for &rel in mednafen_root_c { c_build.file(vendor.join(rel)); }
    for &rel in pce_fast_c { c_build.file(vendor.join(rel)); }
    for &rel in sound_c { c_build.file(vendor.join(rel)); }
    for &rel in libretro_common_c { c_build.file(vendor.join(rel)); }
    for &rel in libchdr_c { c_build.file(vendor.join(rel)); }
    for &rel in zlib_c { c_build.file(vendor.join(rel)); }
    for p in &tremor_c { c_build.file(p); }

    c_build.compile("oa_pce_native_c");

    // ---- C++ build ----
    let mut cxx_build = cc::Build::new();
    apply_includes(&mut cxx_build, &vendor, &common_includes);
    apply_defines(&mut cxx_build, common_defines);
    cxx_build.cpp(true).warnings(false);
    // MSVC: enable exceptions for Mednafen's use of throw/try/catch.
    cxx_build.flag_if_supported("/EHsc");
    // Mednafen uses a lot of pre-C++11 idioms. Use C++14 to be safe.
    cxx_build.flag_if_supported("/std:c++14");

    for &rel in mednafen_root_cxx { cxx_build.file(vendor.join(rel)); }
    for &rel in pce_fast_cxx { cxx_build.file(vendor.join(rel)); }
    for &rel in cdrom_cxx { cxx_build.file(vendor.join(rel)); }
    for &rel in libretro_root { cxx_build.file(vendor.join(rel)); }
    // Our own shim layer (lives in the crate root, not under vendor/).
    cxx_build.file("shim.cpp");
    println!("cargo:rerun-if-changed=shim.cpp");

    cxx_build.compile("oa_pce_native");
}

fn apply_includes(b: &mut cc::Build, vendor: &Path, includes: &[&str]) {
    for inc in includes {
        if inc.is_empty() {
            b.include(vendor);
        } else {
            b.include(vendor.join(inc));
        }
    }
}

fn apply_defines(b: &mut cc::Build, defines: &[(&str, Option<&str>)]) {
    for (k, v) in defines {
        b.define(k, *v);
    }
}

/// Non-recursive list of files in `dir` with the given extension.
fn read_dir_files(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for entry in rd.flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) == Some(ext) {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}
