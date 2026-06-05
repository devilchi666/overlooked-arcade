# Dynamic Controller Info — Consume `RETRO_ENVIRONMENT_SET_CONTROLLER_INFO`

**Status:** queued (Slice 1 ready to start)
**Branch:** `feat/dynamic-controller-info`
**Drives operator validation of:** every light-gun system (NES Zapper, SMS Light Phaser, SNES Super Scope, PSX GunCon, Saturn Virtua Gun, Dreamcast Light Gun, Atari 7800 XEGS Light Gun) + every non-trivial peripheral set (Arkanoid paddle, Power Pad, SNES Mouse, multitaps, DOSBox keyboard, Wii peripherals).

---

## Problem

The per-game Input dialog dispatches the libretro device id the operator picks from a hardcoded dropdown (`DEVICE_ID_OPTIONS_BASE` / `_GAMECUBE` / `_SNES` in `frontend/src/components/GameDialogs.tsx`). Each core has its own subclass ids — and the dropdown's generic `Light Gun = 4`, `Mouse = 2`, `Pointer = 6` entries are wrong for almost every core that uses peripherals.

### Concrete evidence (2026-06-05 light-gun bring-up)

Operator wired Duck Hunt port 1 = "Zapper" (id 4) via the dialog. The dispatch reached FCEUmm correctly (`set_port_device(1, 4) for nes` in the log). FCEUmm's `update_nes_controllers` switch:

```c
case RETRO_DEVICE_ZAPPER:      /* 258, not 4 */
    FCEUI_SetInput(port, SI_ZAPPER, …);
    break;
case RETRO_DEVICE_GAMEPAD:
default:
    FCEUI_SetInput(port, SI_GAMEPAD, …);  /* ← id=4 lands here */
    break;
```

FCEUmm silently wired port 1 as GAMEPAD. The one-shot LIGHTGUN-poll diagnostic in `cb_input_state` never fired — the core never even asked for LIGHTGUN coordinates.

Same shape across every core researched this session (verified by reading each core's `libretro.c`):

| Core | Peripheral | Constant | Wire id |
|---|---|---|---|
| FCEUmm (NES) | Zapper | `SUBCLASS(MOUSE, 0)` | **258** |
| FCEUmm (NES) | Arkanoid | `SUBCLASS(MOUSE, 1)` | 514 |
| FCEUmm (NES) | Power Pad A / B | `SUBCLASS(KEYBOARD, 0/1)` | 259 / 515 |
| snes9x (SNES) | Super Scope | `(1 << 8) \| LIGHTGUN` | **260** |
| snes9x (SNES) | Justifier 1 / 2 | `(2/3 << 8) \| LIGHTGUN` | 516 / 772 |
| snes9x (SNES) | MACS Rifle | `(4 << 8) \| LIGHTGUN` | 1028 |
| Genesis Plus GX (SMS / MD) | Light Phaser | `SUBCLASS(LIGHTGUN, 0)` | **260** |
| Genesis Plus GX (MD) | Menacer | `SUBCLASS(LIGHTGUN, 1)` | 516 |
| Genesis Plus GX (MD) | Justifiers | `SUBCLASS(LIGHTGUN, 2)` | 772 |
| Beetle PSX | GunCon | `SUBCLASS(LIGHTGUN, 0)` | **260** |
| Beetle PSX | Justifier | `SUBCLASS(LIGHTGUN, 1)` | 516 |

The id-space is `((sub_id + 1) << 8) | base_id` for the canonical `RETRO_DEVICE_SUBCLASS` macro, but several cores (snes9x, Dolphin) roll their own `(N << 8) | base` without the `+1`. Hardcoding any of this per-core in the frontend is band-aid territory — we'd have to research, encode, and ship per-system tables for every core and re-do the work every time a core update changes subclass numbering.

### Why we don't need to hardcode anything

libretro already publishes this. Every core that supports non-trivial peripherals calls `RETRO_ENVIRONMENT_SET_CONTROLLER_INFO` during init with a null-terminated array of `retro_controller_info` (one per port), each carrying a `retro_controller_description[]` listing every supported device by `(desc, id)`. The descriptions use the core's own language ("Zapper", "Light Phaser", "GunCon") so the dropdown reads correctly without manual translation.

Our current env handler (`crates/oa-libretro/src/state.rs:1216`) returns `true` for this env command and discards the data. The whole arc is about consuming it.

### Why this arc not band-aids

After Slice 4 deletes the hardcoded tables, adding a new core requires zero frontend work — its peripherals appear automatically, labeled by the core author. Subclass numbering changes upstream propagate without intervention. The same data drives `LIGHT_GUN_SYSTEM_IDS` derivation (today hand-listed in `frontend/src/components/LightGunHelp.tsx:15`) and unlocks future per-system / per-game peripheral-aware UI.

Memory: `feedback_no_bandaid_fixes` codifies the operator's preference for arcs like this over per-system patch lists.

---

## Spec reference

From `libretro.h` (vendored at `crates/oa-pce-sys/vendor/libretro-common/include/libretro.h`):

```c
#define RETRO_ENVIRONMENT_SET_CONTROLLER_INFO  35

struct retro_controller_description {
    const char *desc;       /* human-readable, e.g. "Zapper" — core-owned static C string */
    unsigned id;            /* libretro device id (base or subclass) */
};

struct retro_controller_info {
    const struct retro_controller_description *types;
    unsigned num_types;
};
```

The env data argument is `const struct retro_controller_info *` pointing to an array of `retro_controller_info`. The array terminator is the sentinel `{ types == NULL, num_types == 0 }`. There is NO explicit count — walk until the sentinel.

The `desc` strings are typically into the core's `.dll` text segment (valid for the core's lifetime per the spec; we clone to owned `String` at parse time to decouple frontend lifetime from core lifetime).

The env call may fire BEFORE `retro_load_game` (during `retro_set_environment` or `retro_init`) OR AFTER. Both timings need to work. Most cores call it before load.

---

## Implementation slices

### Slice 1 — Rust: parse `SET_CONTROLLER_INFO` + accessor

Files:
- `crates/oa-libretro/src/state.rs`
- `crates/oa-libretro/src/core.rs`

Add `DeviceDescriptor { label: String, id: u32 }` (re-exportable). In the env handler, replace the bare `true` return for `SET_CONTROLLER_INFO` with parsing:
1. Cast `data` to `*const retro_controller_info`.
2. Walk until sentinel; for each entry, walk its `retro_controller_description[num_types]`.
3. Clone each `desc` CStr to owned `String`.
4. Store as `Vec<Vec<DeviceDescriptor>>` in the singleton state (keyed by port index).
5. Log at INFO with port count + per-port device list — gives operator-readable trace.
6. Return `true`.

Add `LibretroCore::controller_devices(port: u32) -> Vec<DeviceDescriptor>` accessor.

Tests:
- Synthetic `retro_controller_info` arrays with 1, 2, 5 ports.
- Sentinel-termination handling.
- Empty per-port lists.
- String cloning correctness (mutate source after parse, assert stored copies unchanged).
- Out-of-bounds port returns empty vec.

Estimated: ~50 LOC + ~100 LOC tests.

### Slice 2 — Tauri command + frontend live-game path

Files:
- `apps/oa-shell/src/main.rs`
- `frontend/src/components/GameDialogs.tsx`

New Tauri command:
```rust
#[tauri::command]
fn get_controller_devices(port: u32, state: tauri::State<'_, AppState>)
    -> Result<Vec<DeviceDescriptor>, String>
```
Reads from the live core if loaded; returns empty vec if no core or port out of range.

Frontend:
- New `useControllerDevices(port)` hook — Solid resource that calls `invoke("get_controller_devices", { port })`.
- Refactor `GameInputDialog`'s port-0 dropdown + the additional-ports dropdowns to consume the resource instead of `deviceOptionsForSystem`.
- Fallback when the resource returns empty: render base list (`Standard Pad` id=1, `Disconnected` id=0) with a hint "Launch the game to see this core's full device list."
- Keep the existing `DEVICE_ID_OPTIONS_BASE` / `_GAMECUBE` / `_SNES` constants in place for Slice 2 — they're the fallback list. Slice 4 deletes them.

Estimated: ~80 LOC across both ends.

### Slice 3 — SQLite cache + non-live dialog path

Files:
- `apps/oa-shell/src/library_db.rs` (schema bump v20 → v21, new table + accessors)
- `apps/oa-shell/src/main.rs` (populate cache on core load, read from cache in command)

Schema:
```sql
CREATE TABLE core_controller_info (
    core_filename  TEXT NOT NULL,    -- e.g. "fceumm_libretro.dll"
    port           INTEGER NOT NULL, -- 0..=4
    devices_json   TEXT NOT NULL,    -- JSON array [{label, id}, ...]
    captured_at    INTEGER NOT NULL, -- unix seconds at last write
    core_mtime     INTEGER NOT NULL, -- .dll mtime at capture; invalidates cache when .dll changes
    PRIMARY KEY (core_filename, port)
);
```

Behavior:
- Every successful `core.load_rom` triggers a write: pull `controller_devices(port)` for each of 5 ports, upsert.
- `get_controller_devices` command falls through to cache when no core is loaded (or the loaded core isn't this game's system's default core).
- Cache invalidation: when a core is loaded, compare its file mtime against `core_mtime` for the cached row; if different, the cache for that core is rewritten (the live load is the source of truth).

Tests:
- Cache write on core load.
- Cache read when no core loaded.
- mtime invalidation.
- Migration v20 → v21.

Estimated: ~80 LOC + ~60 LOC tests.

### Slice 4 — Delete the hardcoded fallback tables + legacy-id label

Files:
- `frontend/src/components/GameDialogs.tsx` — delete `DEVICE_ID_OPTIONS_BASE` / `_GAMECUBE` / `_SNES`, `systemSpecificDeviceLabel`, `deviceOptionsForSystem`.
- `frontend/src/components/LightGunHelp.tsx` — replace `LIGHT_GUN_SYSTEM_IDS` hand-list with `isLightGunSystem(systemId)` that calls a new `system_has_light_gun` Tauri command. That command checks the cached `core_controller_info` for any device whose `id & 0xFF == RETRO_DEVICE_LIGHTGUN`.

Legacy-id labels in the dialog: when the saved override id isn't in the advertised list, render as `Unknown device (id X) — re-pick`. The user's existing port1=4 override from this morning's session falls into this case until they re-pick — no migration script needed.

Estimated: ~60 LOC deletions + ~40 LOC for the lightgun-detection command and legacy label.

### Slice 5 — Operator validation pass

Operator launches Duck Hunt with the new dynamic dropdown. FCEUmm's advertised devices show up (Zapper, Power Pad, Arkanoid, etc.). Operator picks Zapper from the now-correct list. Mouse aim + left-click trigger work.

If Duck Hunt validates green, the other 6+ light-gun systems get a quick smoke test (SMS Phaser via Genesis Plus GX is the easiest second since the core is already shipped). Each closed validation flips the matching ⬜ to ✅ in `docs/cores/<id>/ROADMAP.md` per CLAUDE.md hygiene rule. Per-core READMEs get a "Light gun validation: shipped 2026-06-XX via dynamic-controller-info" line.

Merge `--no-ff` to main after operator thumbs-up. Branch deleted both sides per `feedback_branch_workflow`.

---

## Trade-offs to know about

- **First dialog open per core needs a cache populated.** Until the core has been loaded once on this install, the dropdown falls back to the base list with a hint. This matches RetroArch's behavior — no surprise here.
- **Some cores never call `SET_CONTROLLER_INFO`.** Rare in modern cores but possible. Fallback list handles it; nothing breaks.
- **String desc lifetime is per-core, but we clone at parse time.** Frontend hop is safe; the only risk is if a core mutates its own descriptions between env call and our parse — no shipping core does this.
- **Schema bump is per-install only.** Existing installs migrate forward on next launch; no operator action needed.
- **The mirror-pointer-to-ports-1-4 fix from `ee0f813`** is a prerequisite and already on main. Without it, even a correctly-dispatched LIGHTGUN to port 1 reads `(0, 0)` because the pointer state never flows. Dynamic-controller-info plus mirror-pointer together unlock the full light-gun pipeline.

---

## Out of scope (parked, not for this arc)

- Per-core peripheral validation matrix (separate operator-validation arc per system).
- Light-gun calibration UI (the per-system viewport mapping + sensitivity tuning — separate feature).
- Multi-controller live polling (only port 0 is polled today; second-player support is a separate refactor).
- Dynamic core-option dropdowns (same shape applies to `RETRO_ENVIRONMENT_SET_CORE_OPTIONS` — we already parse those; the analog of this arc for core options shipped as the existing core-options pipeline).

---

## Memory hooks

- `feedback_no_bandaid_fixes` — codifies the operator preference that drove this arc.
- `feedback_defer_plans_to_intree_docs` — this doc is the durable plan if the arc spans sessions.
- `feedback_branch_workflow` — feat/dynamic-controller-info branch; merge --no-ff after validation.
- `reference_libretro_controller_after_load_game` — Mednafen-derived cores need `set_controller_port_device` AFTER `retro_load_game`. Our existing `arm_libretro_device` already does this; relevant to remember when wiring Slice 3's "populate cache on load" path.
