# sms — Roadmap

Per-core phase tracking for Sega Master System. Mirrors the project-wide
ROADMAP shape (Phase 0 = onboarded, Phase 1 = first ROM running, Phase 2
= polish, Phase 3+ = shared infra) but scoped to SMS.

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-19)

Core comes online via the libretro pivot — no Rust crate vendoring.
Genesis Plus GX installed by operator; OA wires the system into the
existing shell, scanner, bindings, library DB, and settings pipelines.

- ✅ System registered in `frontend/src/themes/registry.ts` — `SystemId`
  union extended with `sms`, `systemThemes.sms` entry (extension `sms`,
  landscape tile aspect 4/3, `crt-lite` default shader preset).
- ✅ Theme block in `frontend/src/themes/systems.css` — neon magenta
  (hue 340°, chroma 0.22), evoking the 1986-1990 Western Big Box
  grid-floor art.
- ✅ Per-system button bits + bindings in `apps/oa-shell/src/bindings.rs::sms`
  — 7-button layout (4-way d-pad + B1 + B2 + PAUSE), `SMS_BUTTONS`
  table, `default_sms_bindings()`, `defaults_for("sms")` arm.
- ✅ `sms_to_libretro_bits` identity remap (bits laid out to match
  libretro RetroPad positions directly).
- ✅ `bindings::to_libretro_bits` + `bit_for` + `buttons_for` dispatch
  arms include `"sms"`.
- ✅ `default_core_dll_for_system("sms") → "genesis_plus_gx_libretro.dll"`
  in `apps/oa-shell/src/main.rs`. `parse_system_id("sms") → SystemId::Sms`
  (already wired from a prior session).
- ✅ `rom_hashes::libretro_dat_refs_for_system("sms")` returns
  `&[DatRef { subdir: "metadat/no-intro", basename: "Sega - Master System - Mark III" }]`.
- ✅ `media::repo_for_system_id("sms")` returns
  `Some("Sega_-_Master_System_-_Mark_III")` (was wired ahead of onboarding;
  test fixture bumped to include `sms` in the onboarded set).
- ✅ Per-core docs scaffold (this directory).

**Acceptance gate:** Operator can drop `genesis_plus_gx_libretro.dll`
into the install, scan an SMS ROMs folder, see SMS-themed (neon magenta)
tiles appear in the library, and click one to launch — without rebuilding
Rust.

---

## ⬜ Phase 1 — First SMS ROM running

- ⬜ Operator validation: launch a real `.sms` ROM end-to-end (pixels +
  audio + controller). Suggested reference set: **Alex Kidd in Miracle
  World**, **Phantasy Star**, **Wonder Boy III: The Dragon's Trap**,
  **Sonic the Hedgehog (SMS)**, **Shinobi**.
- ⬜ Save state F5/F8 round-trip confirmation via the existing path —
  Genesis Plus GX supports `retro_serialize` for SMS state.
- ⬜ Multi-region testing: load USA + Europe + Japan (Mark III) dumps to
  confirm region auto-detect drives the right NTSC 59.92 Hz vs PAL
  49.70 Hz timing.
- ⬜ Per-game cover sync via libretro-thumbnails — **infra ready
  2026-05-19, needs operator validation.** Operator: run
  `Settings → Library → Sync media for SMS` and confirm covers download.
- ⬜ Libretro-database hash matching — same — operator runs
  `Settings → Library → Identify ROMs` to confirm No-Intro SHA-1 lookup
  populates canonical titles + publishers + years.
- ⬜ Japan-region FM sound (YM2413) compatibility — Japanese SMS / Mark
  III games (some Sega titles, Wonder Boy III JP) gain extra FM tracks
  when the optional FM Sound Unit is enabled. GPGX surfaces this via a
  core option; needs operator validation that the per-system Core
  Options surface exposes it cleanly.

**Acceptance gate:** A reference set of SMS games run with pixels +
audio + working controller at native 59.92 Hz NTSC.

---

## ⬜ Phase 2 — Polish

- ⬜ Per-system shader tweaks: SMS games shipped on CRTs; `crt-lite` is
  the registered default but operator may want to confirm it reads
  correctly against the SMS palette (Phantasy Star's spritework,
  Wonder Boy III's saturated parallax).
- ⬜ Optional BIOS handling — surface `bios.sms` presence / absence in
  the per-system Settings page so operators understand the "skip boot
  splash" behavior is opt-in.
- ⬜ Light Phaser (SMS's light-gun peripheral) — deferred. Same analog-
  input dependency as Atari 7800 Trak-Ball / Robotron twin-stick;
  picks up once shared light-gun infra lands.
- ⬜ 3D glasses (SMS SegaScope add-on) — deferred. Genesis Plus GX
  supports the 3D mode via anaglyph fallback, but the OA frontend
  doesn't surface the toggle yet.

---

## ⬜ Phase 3+ — Stretch

Per the project ROADMAP, all post-Phase-3 work (rewind, TAS, WebM export,
memory inspector, cheats, milestones, run-ahead) is system-agnostic and
lights up automatically once the engine work ships. SMS-specific items:

- ⬜ Game Genie / Pro Action Replay code support — runs through the
  libretro cheat path (project RetroArch parity slice 8); needs
  validation that Genesis Plus GX's `retro_cheat_set` accepts SMS
  Game Genie format.
- ⬜ Custom forked Genesis Plus GX — only if upstream regresses or we
  want OA-specific SMS extensions. Recipe mirrors the Beetle PCE Fast
  plan: separate libretro-frontend build of our patched source that
  emits a .dll we ship in the installer.

---

## Scope clarifications

- **No vendoring for SMS today.** The libretro pivot means we ship the
  upstream nightly Genesis Plus GX .dll alongside our binary and tell
  operators to drop it into `<exe_dir>/cores/`.
- **No BIOS required.** SMS cart playback is BIOS-optional — boot splash
  is the only thing affected. `bios.sms` in `<exe_dir>/system/` is the
  per-system convention if the operator wants the era-correct boot.
- **`.bin` extension intentionally excluded** to avoid collision with
  PCE-CD track files, Sega CD audio tracks, ColecoVision, and Atari
  2600 dumps. Users with `.bin` SMS dumps rename to `.sms`.
- **Shared .dll with Game Gear.** One Genesis Plus GX install services
  both slugs — operators installing for one get the other for free.

---

## 2026-05-21 — Stale-cleanup audit

The Phase 1+ items above were written when this system onboarded, before cross-system infrastructure (Phases 1.5 / 2.5–2.8 / 3 / 4 + direct-launch CLI) landed. Many `⬜` items are actually shipped — see `docs/cores/AUDIT_2026-05-21.md` for the per-item breakdown (stale vs open-code vs open-operator) for this system.
