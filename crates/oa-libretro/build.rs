// Compiles src/log_trampoline.c into a static archive linked into this crate.
// The trampoline bridges libretro's variadic `void(level, fmt, ...)` log_cb
// (which Rust can't express on stable — c_variadic is unstable) to a normal
// extern "C" Rust forwarder that lands the formatted message in our log crate.

fn main() {
    println!("cargo:rerun-if-changed=src/log_trampoline.c");
    cc::Build::new()
        .file("src/log_trampoline.c")
        .warnings(true)
        .compile("oa_libretro_log_trampoline");
}
