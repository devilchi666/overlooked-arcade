# Dynamic Input Descriptors — Consume `RETRO_ENVIRONMENT_SET_INPUT_DESCRIPTORS`

**Status:** queued (Slice 1 ready to start)
**Branch:** `feat/dynamic-input-descriptors`
**Drives:** per-game button-label visibility in the per-system Bindings page + the per-game Input dialog. Operator stops having to remember "what does B do in this game?" — the core's authored answer renders inline.
**Sibling arc:** `docs/PLANS/dynamic-controller-info.md` — same shape, same machinery, same operator preference per `feedback_no_bandaid_fixes`.

---

## Problem

`crates/oa-libretro/src/state.rs:1120` ack's `RETRO_ENVIRONMENT_SET_INPUT_DESCRIPTORS` (env 11) and discards the data:

```rust
RETRO_ENVIRONMENT_SET_INPUT_DESCRIPTORS => true,
```

The data is a null-terminated array of:

```c
struct retro_input_descriptor {
    unsigned port;
    unsigned device;
    unsigned index;
    unsigned id;
    const char *description;     // human-readable label, e.g. "Jump"
};
```

(See `crates/oa-libretro/src/ffi.rs` for the live ABI typedefs:
`retro_input_descriptor` (line ~361) and the
`RETRO_ENVIRONMENT_SET_INPUT_DESCRIPTORS = 11` env constant (line ~163).
The old `crates/oa-pce-sys/vendor/.../libretro.h` reference is retired —
`oa-pce-sys` is excluded from the workspace build.)

Cores publish these arrays **per-game**, not per-system — FCEUmm describes Super Mario Bros' B button as "Run" and Castlevania's B button as "Whip" because the in-game semantics differ. The bindings UI currently shows OA's per-system RetroPad bit labels ("A", "B", "X", "Y", "L", "R", "Start", "Select"), which are the *physical* mapping the operator chose. The per-game *semantic* label is what the core publishes — and what the operator actually wants to see when remapping.

Without consuming this, the operator's mental model is "I remember B was the run button in Mario" instead of "the bindings table tells me right there." For systems with sprawling games-with-different-semantics (NES, SNES, Genesis, every fighting game system), the UX is meaningfully better with this consumed.

### Why this is exactly the same shape as the light-gun band-aid

- Core publishes data via `RETRO_ENVIRONMENT_SET_*`.
- We ack-and-discard.
- Frontend hardcodes a worse version (the per-system "A / B / X / Y" labels are intentional + correct as RetroPad-bit labels, but they're incomplete — they don't carry game semantics).

The fix uses the same machinery as `dynamic-controller-info`: parse the null-terminated array in `state.rs`, clone strings (lifetime per-spec is "until `retro_unload_game()`" — same decoupling story as `SET_CONTROLLER_INFO`'s `desc` field), store in singleton state, expose via accessor + Tauri command, render in the dialog, cache in SQLite for the pre-launch case.

### Why this matters more than "polish"

- **Multi-game systems where one binding means many things.** Fighting games dominate this — Street Fighter II's `Y/B/A` on SNES are "Light Punch / Medium Punch / Heavy Punch" but Super Mario World's are "Jump / Spin / X-action". Operator currently has to read the manual. Cores already authored the answer.
- **Foundation for future polish.** A core's published descriptor is the source of truth for "what does THIS bit do in THIS game" — drives any future "rebind by semantic name" UI ("set Jump to spacebar"), per-game key-symbol overlays, glyphs on controller diagrams, etc.
- **No content-curation burden.** Unlike `games.yaml` per-game notes, this data is already authored upstream by core maintainers. We just need to surface it.

---

## Spec reference

```c
#define RETRO_ENVIRONMENT_SET_INPUT_DESCRIPTORS 11

struct retro_input_descriptor {
    unsigned port;
    unsigned device;
    unsigned index;
    unsigned id;
    const char *description;     // strings live "until retro_unload_game() is called"
};
```

Array terminator: `description == NULL` (no explicit count — walk until sentinel).

Per the spec docstring (libretro.h:582-590):
> "It is up to the frontend to present this in a usable way. […] This function can be called at any time, but it is recommended to call it as early as possible."

So cores may call it during `retro_set_environment`, `retro_init`, or `retro_load_game`. By the time we cache post-load (same point as Slice 3 of the controller-info arc), all calls have fired.

`(port, device, index, id)` is the lookup key — `device` is one of `RETRO_DEVICE_JOYPAD` / `_ANALOG` / `_MOUSE` / etc.; `index` distinguishes analog-stick LEFT vs RIGHT (0/1); `id` is the per-device bit (`RETRO_DEVICE_ID_JOYPAD_A` etc. for JOYPAD; `RETRO_DEVICE_ID_ANALOG_X`/`_Y` for ANALOG). Frontend lookup will be `descriptors_for_game.get(&(port, device, index, id))`.

---

## Implementation slices

### Slice 1 — Rust: parse `SET_INPUT_DESCRIPTORS` + accessor

Files:
- `crates/oa-libretro/src/ffi.rs` — add `retro_input_descriptor` struct (5 fields, `#[repr(C)]`).
- `crates/oa-libretro/src/state.rs` — replace the env-11 bare `true` with a parser; store as `Vec<InputDescriptor>` in singleton State.
- `crates/oa-core/src/lib.rs` — add system-agnostic `InputDescriptor { port: u32, device: u32, index: u32, id: u32, description: String }` shape (mirrors `ControllerDeviceDescriptor` convention).
- `crates/oa-libretro/src/core.rs` — `LibretroCore::input_descriptors() -> Vec<InputDescriptor>` accessor.
- `crates/oa-libretro/src/lib.rs` — `pub use state::loaded_core_input_descriptors`.

Implementation notes:
- Walker mirrors `parse_controller_info` shape — `loop { read; if sentinel break; consume; advance; defensive cap at N }`. Cap at 256 descriptors (way more than any real game has — Street Fighter at the upper end declares ~30).
- Clone `description` to owned `String` so the frontend hop doesn't depend on core lifetime.
- Log an INFO line at parse time with descriptor count: `oa-libretro: SET_INPUT_DESCRIPTORS — N descriptors received`. Truncated preview helper similar to `format_controller_devices_for_log` (don't dump all of them; show first ~5 with "+M more" tail).

Tests (`#[cfg(test)] mod tests`):
- Null top pointer → empty Vec.
- Sentinel walking → stops cleanly.
- NULL description → empty `String` (don't panic; spec doesn't explicitly say NULL is invalid).
- String ownership across source drop (mirrors the controller-info test).
- Multiple descriptors at same `(port, device)` with different ids → all preserved in order.

Estimated: ~80 LOC + ~120 LOC tests.

### Slice 2 — Tauri command + frontend live-game path

Files:
- `apps/oa-shell/src/main.rs` — new `get_input_descriptors(systemId?)` Tauri command. Live core wins; falls through to cache (Slice 3) when no live. Register in invoke_handler list.
- `frontend/src/components/SystemBindingsEditor.tsx` — `createResource` fetches descriptors for the system's currently-loaded game (or live core). Render the description string inline next to each RetroPad row when one matches `(0, JOYPAD, 0, bit_id)`. Format example: `[Y] Whip — bound to: Z`. Falls back to no-suffix display when no descriptor matches.
- `frontend/src/components/GameDialogs.tsx` (Input dialog) — similar treatment for the port-N dropdowns. After the dropdown, render any descriptors for that port that don't match the standard JOYPAD bits (analog-stick descriptions, mouse-button descriptions, etc.).

Notes:
- Descriptors are per-game. The bindings page is per-system but the operator typically opens it while a specific game is in focus / running — use the running entry's id as the cache key. If no game is in focus, fall back to showing no descriptors (the per-system bindings still render correctly).
- Don't replace the existing per-system bit labels — they're the *physical* mapping name (the canonical RetroPad button) and stay correct. The descriptor is the *semantic* in-game label. UI shows both: `[Y physical button] · [core: "Whip"]`.

Estimated: ~100 LOC.

### Slice 3 — SQLite cache + pre-launch path

Files:
- `apps/oa-shell/src/library_db.rs` — schema bump v21 → v22. New table:

```sql
CREATE TABLE IF NOT EXISTS core_input_descriptors (
    core_filename   TEXT NOT NULL,
    game_sha1       TEXT NOT NULL,    -- per-game key (NOT per-core like controller-info)
    descriptors_json TEXT NOT NULL,
    captured_at     INTEGER NOT NULL,
    core_mtime      INTEGER NOT NULL,
    PRIMARY KEY (core_filename, game_sha1)
);
```

Key difference from `core_controller_info`: this is keyed by `(core_filename, game_sha1)` since the descriptors are PER-GAME. Same mtime-invalidation pattern as controller-info — when the .dll's mtime changes, cached descriptors are stale and the next load refreshes.

- `apps/oa-shell/src/main.rs` — on every successful `core_ref.load_rom`, after we cache controller-info, also cache input descriptors keyed by `(current_core_dll, current_rom_sha1)`. SHA-1 is available from the launched-rom resolution path (the games table already carries it).
- `get_input_descriptors` Tauri command falls through to cache when no live core, keyed by `(effective_core_for_system, game_sha1)`.

Tests (in `library_db.rs::tests`):
- Cache round-trip with mtime invalidation.
- Per-(core_filename, game_sha1) keying — two games on the same core round-trip independently.
- Empty descriptors persist correctly.

Estimated: ~80 LOC + ~60 LOC tests.

### Slice 4 — Operator validation

NES Super Mario Bros + Castlevania (or any two FCEUmm games with distinct B-button semantics). Open Bindings page per-system while each game is running; descriptors should differ. If green, merge `--no-ff`. Per-core ROADMAPs get an audit-pass line noting per-game labels now flow.

---

## Trade-offs to know about

- **Strings live "until retro_unload_game()" per spec.** Same lifetime constraint as `SET_CONTROLLER_INFO` `desc` strings — we clone at parse time. No new risk.
- **Per-game cache key, not per-core.** Each game gets its own row. A 200-game library on FCEUmm = up to 200 rows. Storage cost is trivial (~100 bytes per row × 200 = ~20 KB) but the schema decision is worth noting.
- **Cores that don't call SET_INPUT_DESCRIPTORS.** Common — many cores skip this. Fallback is "show nothing extra" (the existing per-system bit labels still render). No regression.
- **Dialog UX for "ANALOG" descriptors.** Cores publish descriptors for analog sticks too — e.g., `(port=0, device=ANALOG, index=LEFT, id=X) → "Move horizontal"`. The per-system Bindings page currently doesn't have a row per analog axis; consider how/where to render those. v1 can render them as a separate "Analog inputs" section below the digital table if any are published; v2 can integrate them into the analog routing panel.
- **The `RETRO_ENVIRONMENT_SET_INPUT_DESCRIPTORS` env can fire multiple times during a single game's lifetime** — per spec, "can be called at any time." Some cores re-publish when a game's internal mode changes (e.g. a fighting game's character-select vs in-match). Our parser overwrites on each call, which matches what every other frontend does; the latest publish wins.

---

## Out of scope (parked, not for this arc)

- "Rebind by semantic name" UI ("set Jump to spacebar"). Foundation laid by this arc; UI is a separate feature.
- Per-game key-symbol overlays / glyphs on controller diagrams. Same — foundation laid; rendering is separate.
- Analog-axis descriptor UI integration with the analog routing panel — deferred to v2 as noted.

---

## Memory hooks

- [[feedback-no-bandaid-fixes]] — codifies the operator preference that drove this arc and `dynamic-controller-info`.
- [[feedback-defer-plans-to-intree-docs]] — why this plan lives in `docs/PLANS/` instead of session-scoped notes.
- [[feedback-branch-workflow]] — feat/dynamic-input-descriptors branch; merge --no-ff after operator validation.
- Cross-ref: `docs/PLANS/dynamic-controller-info.md` — the sister arc, identical machinery, fresh in tree.
