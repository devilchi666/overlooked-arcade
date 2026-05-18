/*
 * libretro log_interface trampoline.
 *
 * Cores call retro_log_printf_t with a printf-style format + varargs. Rust
 * on stable can't express that signature (c_variadic is unstable), so we
 * keep the variadic frame on the C side, vsnprintf into a stack buffer, and
 * hand the resolved string to a non-variadic Rust forwarder which routes it
 * through the `log` crate.
 *
 * Linked into the crate via cc in build.rs. Symbol naming uses an `oa_`
 * prefix so it can't collide with anything in libretro-common.
 */

#include <stdarg.h>
#include <stdio.h>

/* Implemented in src/state.rs with #[no_mangle] extern "C". */
extern void oa_libretro_log_forward(unsigned level, const char *msg);

/*
 * Function pointer handed to the core via retro_log_callback.log. C side
 * keeps the real variadic signature; the core uses it as-is.
 *
 * 2 KB stack buffer is enough for any libretro-core message I've ever seen
 * (Mednafen tops out around 200 chars); truncation is preferable to a heap
 * allocation in this hot path.
 */
void oa_libretro_log_trampoline(unsigned level, const char *fmt, ...)
{
    char buf[2048];
    va_list ap;
    va_start(ap, fmt);
    vsnprintf(buf, sizeof(buf), fmt, ap);
    va_end(ap);
    buf[sizeof(buf) - 1] = '\0';
    oa_libretro_log_forward(level, buf);
}
