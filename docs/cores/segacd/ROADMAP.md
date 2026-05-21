# segacd — Roadmap

Per-core phase tracking for Sega CD / Mega-CD. Mirrors the project-wide
ROADMAP shape but scoped to Sega CD.

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-20)

Paired with Sega 32X. Core comes online via the libretro pivot — no
Rust crate vendoring. Genesis Plus GX is the recommended default
(same .dll already shipping for SMS + Game Gear).

- ✅ System registered in `frontend/src/themes/registry.ts` — `SystemId`
  union extended with `segacd`, `systemThemes.segacd` entry (extensions
  `cue / chd / ccd / toc / m3u / iso`, landscape tile aspect 4/3,
  `plain` default shader preset for FMV-heavy library).
- ✅ Theme block in `frontend/src/themes/systems.css` — sapphire blue
  at hue 235° + L=0.55 + C=0.20. Family-cousin to Genesis cobalt (245°)
  but visually distinct via lightness axis.
- ✅ Per-system input wiring — segacd shares the 6-button Mega Drive
  controller via the `"genesis" | "segacd" | "sega32x" => genesis_*`
  dispatch arms in `apps/oa-shell/src/bindings.rs`. Same pattern PCE-CD
  uses to share TG-16's controller.
- ✅ `default_core_dll_for_system("segacd") → "genesis_plus_gx_libretro.dll"`
  in `apps/oa-shell/src/main.rs`.
- ✅ `parse_system_id("segacd" | "sega-cd" | "mega-cd" | "megacd" | "mcd") →
  SystemId::SegaCd` (new variant on `oa_core::SystemId` enum).
- ✅ `rom_hashes::libretro_dat_refs_for_system("segacd")` returns `&[]`
  with NO_DAT_SYSTEMS entry — CD images aren't single-file SHA-1
  matched; disc-id extraction via `cd_id.rs` deferred to Phase 2.
- ✅ `media::repo_for_system_id("segacd")` returns
  `Some("Sega_-_Mega-CD_-_Sega_CD")` so cover sync works as soon as
  the operator runs it.
- ✅ BIOS pre-check via `check_sega_cd_bios` in main.rs — six canonical
  SHA-1 entries across US / JP / EU regional variants. CD-launch path
  branches by system_id to call the right BIOS check (PCE-CD or Sega CD).
- ✅ Per-core docs scaffold (this directory).

**Acceptance gate:** Operator can drop `genesis_plus_gx_libretro.dll`
into the install + a regional Sega CD BIOS in `<exe_dir>/system/`, scan
a Sega CD ROMs folder (marked via Import Wizard), see Sega CD-themed
(sapphire blue) tiles appear in the library, and click one to launch —
without rebuilding Rust.

---

## ⬜ Phase 1 — First Sega CD game running

- ⬜ Operator validation: launch a real CD image end-to-end (pixels +
  CDDA + controller). Suggested reference set: **Sonic CD** (US v2.00),
  **Lunar: The Silver Star Complete**, **Snatcher**, **Popful Mail**.
  Pick a disc that matches a BIOS region the operator has on hand.
- ⬜ Save state F5/F8 round-trip mid-disc. Should work via libretro
  `retro_serialize` (same path as cart Genesis save states), but CD
  state machinery (CD read-pointer, CDDA buffer) is worth explicit
  smoke-testing.
- ⬜ Multi-region testing: load USA + Europe + Japan CD images with
  matching BIOSes to confirm region auto-detect (NTSC 59.92 Hz vs PAL
  49.70 Hz timing).
- ⬜ CDDA streaming validation — Sonic CD's iconic soundtrack is the
  canonical test for CDDA channel layout + mixing through the libretro
  audio callback.
- ⬜ Per-game cover sync via libretro-thumbnails — **infra ready 2026-05-20,
  needs operator validation.** Mapping `segacd → Sega_-_Mega-CD_-_Sega_CD`
  shipped in `media::repo_for_system_id`. Operator: run `Settings →
  Library → Sync media for Sega CD` and confirm covers download.
- ⬜ Multi-disc title via `.m3u` — Lunar: Eternal Blue's two-disc
  release is the canonical test for the libretro disc-control extension.

**Acceptance gate:** A reference set of Sega CD games run with pixels +
CDDA + working controller at native 59.92 Hz NTSC.

---

## ⬜ Phase 2 — Polish

- ⬜ **Disc-id extraction** — `apps/oa-shell/src/cd_id.rs` currently
  handles PCE-CD only. Sega CD discs carry a different game-id signature
  (typically at offset 0x100-0x110 in the data track). Add a Sega CD
  branch to `cd_id.rs` so the library scanner can canonical-title
  Sega CD discs without operator manual matching. Documented in
  `DECISIONS.md`.
- ⬜ **Switch `rom_hashes::libretro_dat_refs_for_system("segacd")`** to
  `&[DatRef { subdir: "metadat/redump", basename: "Sega - Mega CD & Sega CD" }]`
  once disc-id extraction lands — redump's dat populates `game_serials`
  via `parse_libretro_dat`'s serial path, which keys against the
  extracted disc-id rather than file SHA-1.
- ⬜ **Per-game cover sync — operator validation pass.**
- ⬜ **3-button vs 6-button compatibility map.** Most Sega CD games
  shipped before the 6-button pad's 1993 release, so they assume
  3-button hardware. Handful that misbehave with 6-button announce
  (TBD — populate from operator validation) get per-game pad-mode
  override via the per-game Input drawer.
- ⬜ **Sega CD-specific theming polish.** The sapphire palette ships
  v1; per-system page header art / sidebar icon / CD-jewel-case tile
  frame may need Sega CD-specific tweaks once we have titles in the
  library.

---

## ⬜ Phase 3+ — Stretch

Per the project ROADMAP, all post-Phase-3 work (rewind, TAS, WebM
export, memory inspector, cheats, milestones, run-ahead) is system-
agnostic and lights up automatically once the engine work ships.
Sega CD-specific items:

- ⬜ **32X-CD games (e.g. Night Trap 32X, Corpse Killer 32X, Slam City).**
  These layer the 32X cart-slot addon ON TOP of Sega CD — they need
  both the Sega CD BIOS and the 32X .dll. Phase 3+ work because
  PicoDrive's CD-via-32X path needs explicit validation; Genesis Plus
  GX doesn't handle 32X at all. Currently routed to segacd with a
  stacked sega32x per-game override; needs end-to-end testing.
- ⬜ **Game Genie / Pro Action Replay code support** — runs through
  the libretro cheat path (project RetroArch parity slice 8); needs
  validation that Genesis Plus GX's `retro_cheat_set` accepts Sega CD
  Game Genie format.
- ⬜ **Custom forked Sega CD core** — only if upstream regresses or we
  want OA-specific extensions. Recipe mirrors the Beetle PCE Fast plan:
  separate libretro-frontend build of patched source emitting a .dll
  we ship in the installer.

---

## Scope clarifications

- **No vendoring for Sega CD today.** The libretro pivot means we ship
  the upstream nightly Genesis Plus GX .dll alongside our binary. If
  we ever modify the core, we maintain a separate libretro-frontend
  build of our patched source — see project `DECISIONS.md` 2026-05-16
  entry.
- **BIOS REQUIRED.** Unlike cart Genesis, Sega CD playback can't proceed
  without a regional BIOS. The pre-check refuses early with a clear
  error toast naming `bios_CD_U.bin / bios_CD_J.bin / bios_CD_E.bin`
  in `<exe_dir>/system/`.
- **CD extension collision with PCE-CD.** All six CD container
  extensions (`.cue / .chd / .iso / .m3u / .ccd / .toc`) are also
  claimed by PCE-CD. Disambiguation happens at Import Wizard time via
  per-folder hint — same path PCE-CD navigated. Documented in
  `DECISIONS.md`.
