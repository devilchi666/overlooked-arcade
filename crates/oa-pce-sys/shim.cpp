// oa-pce-sys — C++ shim layer.
//
// Beetle PCE Fast (Mednafen PCE Fast) is a libretro core. Its libretro.cpp
// owns the engine + lifecycle; the rest of the world drives it through the
// `retro_*` entry points and a handful of frontend callbacks we provide.
//
// This shim IS the libretro frontend, but mounted inside our Rust binary
// instead of running as a separate executable. It exposes the small C surface
// designed in Spike 3 (`oa_pce_*`) so the idiomatic Rust wrapper in `oa-pce`
// has a stable API to talk to.
//
// Phase 1 scope: enough to load a HuCard, run frames, return RGBA8 pixels +
// stereo audio samples + accept per-port input. Save states, BIOS-needing
// CD games, cheats, and core-options come later.

#include <cstddef>
#include <cstdint>
#include <cstring>
#include <cstdio>
#include <cstdarg>
#include <new>

extern "C" {
#include "libretro.h"
}

// ---------- shim-owned state (Mednafen PCE is singleton anyway) ----------

namespace {
    // PCE's widest mode is 512×242. Reserve a 1024×512 RGBA8 buffer for slack;
    // the real frame fits easily.
    constexpr unsigned FB_MAX_W = 1024;
    constexpr unsigned FB_MAX_H = 512;
    constexpr size_t   FB_BYTES = FB_MAX_W * FB_MAX_H * 4;

    constexpr size_t AUDIO_RING_SAMPLES = 8192 * 2; // L+R interleaved

    uint8_t  g_fb_rgba[FB_BYTES];
    uint32_t g_fb_width  = 256;
    uint32_t g_fb_height = 239;

    int16_t  g_audio[AUDIO_RING_SAMPLES];
    size_t   g_audio_count = 0;

    uint16_t g_input_bits[5] = {0, 0, 0, 0, 0};

    enum retro_pixel_format g_pix_fmt = RETRO_PIXEL_FORMAT_XRGB8888;

    bool g_initialised = false;
    bool g_game_loaded = false;

    // Held across the retro_load_game call so the core can read it back via
    // RETRO_ENVIRONMENT_GET_GAME_INFO_EXT (Beetle's only data-buffer path).
    const uint8_t* g_pending_rom_data = nullptr;
    size_t         g_pending_rom_size = 0;
    struct retro_game_info_ext g_pending_info_ext;
}

// ---------- frontend callbacks ----------

static void cb_video_refresh(const void* data, unsigned width, unsigned height, size_t pitch) {
    if (!data || width == 0 || height == 0) return;
    if (width  > FB_MAX_W) width  = FB_MAX_W;
    if (height > FB_MAX_H) height = FB_MAX_H;

    g_fb_width  = width;
    g_fb_height = height;
    auto* src8 = static_cast<const uint8_t*>(data);
    uint8_t* dst = g_fb_rgba;

    switch (g_pix_fmt) {
        case RETRO_PIXEL_FORMAT_XRGB8888: {
            // src u32 = 0x00RRGGBB on little-endian -> bytes B G R 0.
            // dst RGBA8.
            for (unsigned y = 0; y < height; ++y) {
                auto* row = reinterpret_cast<const uint32_t*>(src8 + y * pitch);
                for (unsigned x = 0; x < width; ++x) {
                    uint32_t p = row[x];
                    *dst++ = static_cast<uint8_t>((p >> 16) & 0xFF); // R
                    *dst++ = static_cast<uint8_t>((p >>  8) & 0xFF); // G
                    *dst++ = static_cast<uint8_t>((p >>  0) & 0xFF); // B
                    *dst++ = 0xFF;
                }
            }
            break;
        }
        case RETRO_PIXEL_FORMAT_RGB565: {
            // src u16 = RRRRR GGGGGG BBBBB.
            for (unsigned y = 0; y < height; ++y) {
                auto* row = reinterpret_cast<const uint16_t*>(src8 + y * pitch);
                for (unsigned x = 0; x < width; ++x) {
                    uint16_t p = row[x];
                    uint8_t r5 = (p >> 11) & 0x1F;
                    uint8_t g6 = (p >>  5) & 0x3F;
                    uint8_t b5 = (p >>  0) & 0x1F;
                    *dst++ = static_cast<uint8_t>((r5 << 3) | (r5 >> 2));
                    *dst++ = static_cast<uint8_t>((g6 << 2) | (g6 >> 4));
                    *dst++ = static_cast<uint8_t>((b5 << 3) | (b5 >> 2));
                    *dst++ = 0xFF;
                }
            }
            break;
        }
        case RETRO_PIXEL_FORMAT_0RGB1555: {
            // src u16 = 0 RRRRR GGGGG BBBBB.
            for (unsigned y = 0; y < height; ++y) {
                auto* row = reinterpret_cast<const uint16_t*>(src8 + y * pitch);
                for (unsigned x = 0; x < width; ++x) {
                    uint16_t p = row[x];
                    uint8_t r5 = (p >> 10) & 0x1F;
                    uint8_t g5 = (p >>  5) & 0x1F;
                    uint8_t b5 = (p >>  0) & 0x1F;
                    *dst++ = static_cast<uint8_t>((r5 << 3) | (r5 >> 2));
                    *dst++ = static_cast<uint8_t>((g5 << 3) | (g5 >> 2));
                    *dst++ = static_cast<uint8_t>((b5 << 3) | (b5 >> 2));
                    *dst++ = 0xFF;
                }
            }
            break;
        }
        default:
            break;
    }
}

static void cb_audio_sample(int16_t left, int16_t right) {
    if (g_audio_count + 2 > AUDIO_RING_SAMPLES) return;
    g_audio[g_audio_count++] = left;
    g_audio[g_audio_count++] = right;
}

static size_t cb_audio_sample_batch(const int16_t* data, size_t frames) {
    size_t samples = frames * 2;
    if (g_audio_count + samples > AUDIO_RING_SAMPLES) {
        samples = AUDIO_RING_SAMPLES - g_audio_count;
    }
    if (samples > 0) {
        std::memcpy(g_audio + g_audio_count, data, samples * sizeof(int16_t));
        g_audio_count += samples;
    }
    return frames;
}

static void cb_input_poll(void) {
    // We push input via oa_pce_set_input; nothing to fetch here.
}

static int16_t cb_input_state(unsigned port, unsigned device, unsigned /*index*/, unsigned id) {
    if (port >= 5) return 0;
    if (device != RETRO_DEVICE_JOYPAD) return 0;
    if (id > 15) return 0;
    return ((g_input_bits[port] >> id) & 1u) ? 1 : 0;
}

// Stub logger — the core may request RETRO_ENVIRONMENT_GET_LOG_INTERFACE.
static void cb_log(enum retro_log_level level, const char* fmt, ...) {
    const char* tag = "INFO";
    switch (level) {
        case RETRO_LOG_DEBUG: tag = "DEBUG"; break;
        case RETRO_LOG_INFO:  tag = "INFO";  break;
        case RETRO_LOG_WARN:  tag = "WARN";  break;
        case RETRO_LOG_ERROR: tag = "ERROR"; break;
        default: break;
    }
    std::fprintf(stderr, "[pce/%s] ", tag);
    va_list ap;
    va_start(ap, fmt);
    std::vfprintf(stderr, fmt, ap);
    va_end(ap);
}

static bool cb_environment(unsigned cmd, void* data) {
    switch (cmd) {
        case RETRO_ENVIRONMENT_SET_PIXEL_FORMAT: {
            if (!data) return false;
            g_pix_fmt = *static_cast<enum retro_pixel_format*>(data);
            return true;
        }
        case RETRO_ENVIRONMENT_GET_LOG_INTERFACE: {
            if (!data) return false;
            auto* cb = static_cast<struct retro_log_callback*>(data);
            cb->log = cb_log;
            return true;
        }
        case RETRO_ENVIRONMENT_GET_GAME_INFO_EXT: {
            if (!data || !g_pending_rom_data || g_pending_rom_size == 0) return false;
            std::memset(&g_pending_info_ext, 0, sizeof(g_pending_info_ext));
            g_pending_info_ext.full_path       = nullptr;
            g_pending_info_ext.archive_path    = nullptr;
            g_pending_info_ext.archive_file    = nullptr;
            g_pending_info_ext.dir             = nullptr;
            g_pending_info_ext.name            = "rom";
            g_pending_info_ext.ext             = "pce";  // HuCard extension; SuperGrafx will need "sgx"
            g_pending_info_ext.meta            = nullptr;
            g_pending_info_ext.data            = g_pending_rom_data;
            g_pending_info_ext.size            = g_pending_rom_size;
            g_pending_info_ext.file_in_archive = false;
            g_pending_info_ext.persistent_data = true;
            *static_cast<const struct retro_game_info_ext**>(data) = &g_pending_info_ext;
            return true;
        }
        case RETRO_ENVIRONMENT_SET_INPUT_DESCRIPTORS:
            // Core is announcing its controller layout; nothing for us to do.
            return true;
        case RETRO_ENVIRONMENT_GET_SYSTEM_DIRECTORY:
        case RETRO_ENVIRONMENT_GET_SAVE_DIRECTORY: {
            // Tell the core "current directory" — HuCard games don't need BIOS,
            // PCE-CD will get a proper path in Phase 5.
            if (!data) return false;
            *static_cast<const char**>(data) = ".";
            return true;
        }
        case RETRO_ENVIRONMENT_GET_VARIABLE: {
            // Reply "not set" for every core option request — the core falls
            // back to compiled-in defaults (which is what we want).
            if (!data) return false;
            auto* var = static_cast<struct retro_variable*>(data);
            var->value = nullptr;
            return false;
        }
        case RETRO_ENVIRONMENT_SET_VARIABLES:
        case RETRO_ENVIRONMENT_SET_CORE_OPTIONS:
        case RETRO_ENVIRONMENT_SET_CORE_OPTIONS_V2:
        case RETRO_ENVIRONMENT_SET_CORE_OPTIONS_V2_INTL:
        case RETRO_ENVIRONMENT_SET_CORE_OPTIONS_DISPLAY:
            return false;
        case RETRO_ENVIRONMENT_GET_VARIABLE_UPDATE: {
            if (!data) return false;
            *static_cast<bool*>(data) = false;
            return true;
        }
        case RETRO_ENVIRONMENT_GET_CAN_DUPE: {
            if (!data) return false;
            *static_cast<bool*>(data) = true;
            return true;
        }
        case RETRO_ENVIRONMENT_GET_LANGUAGE: {
            if (!data) return false;
            *static_cast<unsigned*>(data) = RETRO_LANGUAGE_ENGLISH;
            return true;
        }
        default:
            return false;
    }
}

// ---------- public C surface ----------

struct OaPceCore {
    // Placeholder so we can hand a non-null pointer back; Mednafen is singleton
    // so the real state lives in g_* globals above.
    int placeholder;
};

struct OaPceFrame {
    uint32_t width;
    uint32_t height;
    const uint8_t* pixels;
};

struct OaPceCoreInfo {
    const char* core_name;
    uint32_t version_major;
    uint32_t version_minor;
};

static OaPceCore g_singleton{0};

extern "C" {

OaPceCore* oa_pce_new(void) {
    if (g_initialised) {
        // Singleton: cores can only exist one at a time.
        return &g_singleton;
    }

    retro_set_environment(cb_environment);
    retro_set_video_refresh(cb_video_refresh);
    retro_set_audio_sample(cb_audio_sample);
    retro_set_audio_sample_batch(cb_audio_sample_batch);
    retro_set_input_poll(cb_input_poll);
    retro_set_input_state(cb_input_state);
    retro_init();
    // NOTE: retro_set_controller_port_device must be called AFTER retro_load_game,
    // not here. Beetle's MDFNI_LoadGame re-initializes the core, which resets the
    // PCE input data_ptr[] table. Calling set_controller_port_device pre-load wires
    // input to a buffer that is then disconnected. See oa_pce_load_rom.
    g_initialised = true;
    return &g_singleton;
}

void oa_pce_free(OaPceCore* /*core*/) {
    if (!g_initialised) return;
    if (g_game_loaded) {
        retro_unload_game();
        g_game_loaded = false;
    }
    retro_deinit();
    g_initialised = false;
}

int32_t oa_pce_load_rom(OaPceCore* core, const uint8_t* data, size_t len) {
    if (!core || !g_initialised) return 1;
    if (!data || len == 0) return 1;
    if (g_game_loaded) {
        retro_unload_game();
        g_game_loaded = false;
    }
    // Stash for the core to retrieve via RETRO_ENVIRONMENT_GET_GAME_INFO_EXT
    // — Beetle's retro_load_game routes data-buffer loads through that path.
    g_pending_rom_data = data;
    g_pending_rom_size = len;

    struct retro_game_info info;
    std::memset(&info, 0, sizeof(info));
    info.path = nullptr;
    info.data = data;
    info.size = len;
    info.meta = nullptr;
    bool ok = retro_load_game(&info);

    // Clear the staging pointers after retro_load_game returns. The core has
    // already copied/refcounted what it needs.
    g_pending_rom_data = nullptr;
    g_pending_rom_size = 0;

    if (ok) {
        g_game_loaded = true;
        // Wire controller port 0 to JOYPAD AFTER retro_load_game so PCEINPUT_SetInput
        // points data_ptr[0] at our input_buf[0]. Pre-load calls are clobbered by
        // MDFNI_LoadGame's init pass.
        retro_set_controller_port_device(0, RETRO_DEVICE_JOYPAD);
    }
    return ok ? 0 : 2;
}

void oa_pce_reset(OaPceCore* core) {
    if (!core || !g_game_loaded) return;
    retro_reset();
}

void oa_pce_run_frame(OaPceCore* core) {
    if (!core || !g_game_loaded) return;
    g_audio_count = 0;
    retro_run();
}

OaPceFrame oa_pce_framebuffer(const OaPceCore* /*core*/) {
    OaPceFrame f;
    f.width  = g_fb_width;
    f.height = g_fb_height;
    f.pixels = g_fb_rgba;
    return f;
}

size_t oa_pce_audio_samples(const OaPceCore* /*core*/, int16_t* out, size_t out_cap) {
    if (!out || out_cap == 0) return 0;
    size_t to_copy = g_audio_count < out_cap ? g_audio_count : out_cap;
    std::memcpy(out, g_audio, to_copy * sizeof(int16_t));
    return to_copy;
}

void oa_pce_set_input(OaPceCore* /*core*/, uint32_t port, uint16_t bits) {
    if (port >= 5) return;
    g_input_bits[port] = bits;
}

OaPceCoreInfo oa_pce_info(void) {
    OaPceCoreInfo i;
    i.core_name = "Beetle PCE Fast (Mednafen PCE Fast, vendored)";
    i.version_major = 0;
    i.version_minor = 9;
    return i;
}

} // extern "C"
