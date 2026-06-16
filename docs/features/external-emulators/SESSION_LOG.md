# External Emulators (Depth) — session log

Most recent entry first. Three lines each: **Shipped / Almost / Next.**

---

## 2026-06-16 — Slice 2 (ED2 recipe-update delivery) DELIVERED via oa-packs Slice 5

- **Shipped:** ED2's "update recipes without an OA rebuild" landed as the
  `emulator-recipes` content-pack type, not a standalone updater (per CP5).
  `EmulatorProfiles::load_default` overlays the bundled `config/emulators/`
  baseline with installed packs from `<exe_dir>/emulator-recipes/community/`
  (whole-profile-by-id, last-wins, conflict-recorded), hot-reloaded on pack
  install/update/uninstall/rollback (CP8/CP9). A changed CLI flag now ships
  as a recipe pack through the operator-initiated, sha256-verified pack
  channel — no rebuild. Full detail in
  [features/content-packs/SESSION_LOG.md](../content-packs/SESSION_LOG.md)
  (Slice 5). Closes this arc's Phase 1 Slice 2.
- **Almost:** n/a — delivered whole via the content-pack channel.
- **Next:** Phase 2 — install pipeline (legal gate + "install for me" on one
  Green, already-wired emulator; arc-plan Slice 3).

---

## 2026-06-15 — Slice 1 shipped (schema accretion + ares/bizhawk)

Branch `feat/external-emulator-depth`.

- **Shipped:**
  - **Per-OS `binary_name` map** (ED4). `binary_name` now accepts either
    the original single string OR a `{ windows, macos, linux }` map, via a
    new untagged `BinaryName` enum + `BinaryNameMap` in
    `apps/oa-shell/src/emulator_profiles.rs`. `BinaryName::resolve()`
    picks the current-OS name (single string → always `Some`; a map that
    omits the current OS → `None`, which makes the soft name-check skip).
    Backward-compatible: all 9 existing profiles (bare strings) parse
    unchanged. Consumers updated in `main.rs` (the Settings-UI flatten
    surfaces the resolved current-OS name; the binary-path warn-check
    resolves first and skips when `None`).
  - **`ares.yaml` + `bizhawk.yaml`** — single positional `{content}`
    template (auto-detect; no `--system`). ares: 15 BIOS-free
    auto-detect-safe systems, full per-OS map. bizhawk: 18 systems,
    per-OS map with **macOS omitted** (no native build — the canonical
    "OS-absent → resolve None" case). Both intentionally exclude
    ambiguous-extension (MSX `.rom`) and disc+BIOS systems (PSX/Saturn).
  - **Reserved `--system` seam documented, not built** — doc comment on
    `launch_args_template` describes the future `{system}` token +
    `system_aliases` map for the rare ambiguous case.
  - Extended `all_shipped_profiles_parse_and_hold_invariants` (binary_name
    names ≥1 platform; on Windows every profile resolves a name; ares +
    bizhawk must be present) + 2 new tests for the per-OS map shape.
    `cargo test -p oa-shell` green (848 passed).
  - **`accepts_archives` recipe field (playtest follow-up).** First BizHawk
    playtest failed: every `coleco` launch hit the external path's
    archived-content gate (`main.rs::launch_rom_external`) — a pre-existing
    VL Phase C2 limitation, not new to this slice — because the ROMs are
    `.zip`. BizHawk + ares both load archives natively, so added a
    declarative `accepts_archives: bool` (default false, ED2-aligned): when
    set, the external path hands the **outer archive path** to the emulator
    instead of erroring (it auto-loads the inner ROM). Set true on
    `ares.yaml` + `bizhawk.yaml`. Other profiles stay false (safe). New
    test `archive_capable_profiles_opt_in` + default-false assertion.
- **Playtested + MERGED (2026-06-15):** operator pointed OA at a real
  BizHawk install, set it as the `coleco` default, and launched an
  archived (`.zip`) game from a normal tile — **EmuHawk opened and tried
  to boot** (it then wanted ColecoVision firmware `coleco.rom`, a
  BizHawk-side setting OA never provides — legal posture intact). Slice 1
  demoable acceptance met; branch `feat/external-emulator-depth` merged to
  main. Two playtest follow-ups happened on the branch before merge:
  (1) `accepts_archives` field; (2) decode `<archive>#<inner>` → outer
  archive path on passthrough (the encoded form failed prepare()'s
  is_file check).
- **Almost:** MAME standalone profile — **deferred** (see Next).
- **Next:**
  - **Operator smoke test** — point OA at a real ares/BizHawk install and
    launch a game from a normal tile (Slice 1 demoable acceptance).
  - **MAME deferred, with reason:** a `content_mode` enum alone is not a
    clean ~1-field add — it has no consumer without real content
    resolution (rom-set name extraction + `rompath` config + library
    scanner changes), which is well beyond Slice 1. Adding a dead field
    would be misleading (no-band-aid). The in-process MAME core already
    covers arcade, so the standalone profile waits until that resolution
    work is scoped. Recorded in RESEARCH/external-emulators.md.
  - **Slice 2** — recipe update delivery (rides content-pack infra).

---

## 2026-06-15 — Arc planned (planning session, no code)

- **Shipped:** The External Emulator Depth arc plan
  ([PLANS/external-emulator-depth.md](../../PLANS/external-emulator-depth.md)) —
  3 phases (recipe upgrade + independent updates · install pipeline ·
  extended control toward window-wrapping). 6 decisions (ED1–ED6):
  OA-authored adapters not a third-party SDK; recipes are updatable data
  decoupled from the OA binary (the load-bearing operator constraint);
  install legal gate (Green/Yellow, default Yellow); schema accretion
  (verified ares + BizHawk auto-detect → no per-system args; per-OS
  binary map + MAME content model are the real additions);
  window-wrapping is a deferred north star; control caps are a separate
  axis from D5. Feature folder + research-doc open-items reconciled.
  Verified live: ares `--system` is optional/fallback-only, BizHawk maps
  extension→console on load.
- **Almost:** n/a (planning only).
- **Next:** Slice 1 — schema accretion (per-OS `binary_name` map) +
  author `ares.yaml` + `bizhawk.yaml` + MAME content-model call. Queued
  in NEXT.md HIGH band. Plan → docs → /clear; execution is a fresh
  session.
