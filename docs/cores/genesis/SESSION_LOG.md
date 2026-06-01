# genesis Session Log

Per-core Shipped / Almost / Next log for Sega Mega Drive / Genesis.
Project-wide log lives at `docs/SESSION_LOG.md`.

## 2026-06-01 — MD button-glyph polish (visual 6-button pad reference)

- **Shipped:** `frontend/src/components/GenesisPadReference.tsx` —
  renders the physical 6-button Mega Drive pad layout (X-Y-Z above
  A-B-C + D-pad on the left + Mode/Start in the centre). Each face
  card shows the per-system keyboard / gamepad binding below the
  letter so operators see the spatial relationship the alphabetical
  bindings row table misses (A is "next to B is next to C along the
  bottom"; X is "above A"; etc. — the muscle-memory shape Street
  Fighter II's 1993 6-button pad introduced). Component uses the
  same `get_bindings` Tauri command + per-system data-system accent
  pattern as `KeypadReference` (Coleco). Shared
  `GENESIS_SYSTEMS = {genesis, segacd, sega32x, sega32xcd}` Set
  gates rendering — all four slugs route to the same
  `GENESIS_BUTTONS` table per `apps/oa-shell/src/bindings.rs:1820`,
  so the visual reference is correct for any of them. Mounted in
  both `SystemBindingsEditor` (per-system Bindings dialog) + the
  per-game Input dialog in `GameDialogs.tsx`. `ROADMAP.md` line 70
  flipped ⬜ → ✅. NEXT.md LOWER #10 closed in the same batch.
- **Almost:** Phase 1 operator validation (Sonic / Streets of Rage 2 /
  Phantasy Star IV / Gunstar Heroes playtest) still ⬜.
- **Next:** 3-button vs 6-button game compatibility map (ROADMAP
  Phase 2 line 69) — operator-driven KNOWN_GAME_BUGS curation when
  playtime surfaces real issues.

Merged to main 2026-06-01 as part of
`feat/per-core-followups-and-audit` (bundled with a cross-system
NEXT.md audit pass that closed 3 stale-shipped entries elsewhere).

---

## 2026-05-19 — Phase 0 onboarding

- **Shipped:**
  - `oa_core::SystemId::Genesis` variant + `parse_system_id("genesis")` arm.
  - `bindings.rs::genesis` module — 10-button 6-button-MD layout
    (A/B/C + X/Y/Z + Start + Mode + d-pad), identity libretro remap,
    `GENESIS_BUTTONS` table, `default_genesis_bindings()`, all dispatch
    arms (`bit_for` / `buttons_for` / `to_libretro_bits` / `defaults_for`).
  - `default_core_dll_for_system("genesis") → "clownmdemu_libretro.dll"`
    in main.rs.
  - `media::repo_for_system_id` → `Sega_-_Mega_Drive_-_Genesis` for
    libretro-thumbnails cover sync.
  - `rom_hashes::libretro_dat_refs_for_system` → `metadat/no-intro/Sega - Mega Drive - Genesis`.
  - Frontend `themes/registry.ts` — `SystemId` union extended with
    `"genesis"`, `systemThemes.genesis` entry (extensions
    `["md", "smd", "gen", "68k"]`, landscape 4/3 tile, `crt-lite` default
    shader preset).
  - Frontend `themes/systems.css` — `[data-system="genesis"]` block,
    cobalt blue at hue 245° + chroma 0.22 (distinct from PCE-CD's
    cyan-blue at 220°).
  - Per-core docs scaffold (README + ROADMAP + this log +
    KNOWN_GAME_BUGS + DECISIONS).
- **Almost:** Phase 1 operator validation — needs a real `.md` ROM
  launched end-to-end (Sonic the Hedgehog, Streets of Rage 2,
  Phantasy Star IV are good test cases).
- **Next:** Operator drops `clownmdemu_libretro.dll` into
  `<exe_dir>/cores/`, scans a Genesis ROMs folder, confirms tiles appear
  with cobalt theme, and launches a known-good ROM. Once Phase 1 ✅,
  Phase 2 polish (3-button vs 6-button game map, MD glyphs) opens.
