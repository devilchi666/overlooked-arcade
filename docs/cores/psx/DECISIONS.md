# psx Decisions Log

Append-only. Newest at the bottom. Every entry: **what** we decided,
**when**, **why**, and **what we considered and rejected**.

psx-specific integration choices live here. Project-wide decisions
(engine stack, license, libretro pivot, etc.) live in
`docs/DECISIONS.md`.

---

## 2026-05-20 — Beetle PSX HW default + Beetle PSX SW as catalog peer

**Decision:** Ship Beetle PSX HW (`mednafen_psx_hw_libretro.dll`) as
the default core for `psx`. Beetle PSX SW (`mednafen_psx_libretro.dll`)
is pre-registered as a recommended catalog peer alternate — the
per-system Cores dropdown surfaces both without requiring manual
.dll install. SwanStation (`swanstation_libretro.dll`) available as
an additional alternate.

**Why:** Beetle PSX HW provides the visually-premium PSX experience
(hardware-accelerated Vulkan/OpenGL renderer with upscaling, texture
filtering, PGXP geometry correction). It's the canonical "good-looking
PSX" choice. But the hardware-renderer path depends on the libretro
core successfully obtaining a Vulkan/OpenGL surface from our wgpu host
— our wgpu defaults to DX12 on Windows, and whether libretro can
hand the GL/Vulkan surface to a DX12 host varies by Windows / driver /
GPU combination.

Beetle PSX SW is the bulletproof fallback. Same Beetle PSX upstream
lineage, same BIOS file set, same compatibility profile, but
software-only rendering — no GPU surface dependency. Pre-registering
SW as a catalog peer (alongside HW) means operators who hit the
GL-from-DX12-handoff issue can swap with one click instead of having
to manually download a second .dll. The forward portability cost is
small (one extra entry in the catalog dialog) and the operator UX
gain is meaningful.

**Considered and rejected:**

- **HW-only default with no SW peer registered.** Saves catalog
  complexity but forces operators hitting GL-handoff issues into a
  bug-report → workaround cycle. Defeated.
- **SW-only default.** Bulletproof but visually unsatisfying — PSX's
  era is exactly where HW upscaling pays off most. Defeated.
- **SwanStation as default.** Active modern PSX core (DuckStation-
  derived), similar HW upscaling. Defeated because Beetle PSX HW has
  longer libretro-upstream cadence + Mednafen lineage shared with
  the rest of OA's CD-shape Beetle cores (Saturn, PCE-CD, Virtual
  Boy, Lynx, WonderSwan) — consistency wins.

---

## 2026-05-20 — psx is a separate SystemId

**Decision:** Sony PlayStation games live under a dedicated `psx`
`SystemId`. Distinct sidebar entry, theme (teal cyan 180°), per-system
settings file, library shelf.

**Why:** Standard "every console gets its own home" pattern. PSX is
its own platform with its own controller (DualPad → DualShock),
library, and cultural identity. Sharing a SystemId with another
system would force inappropriate cross-system defaults.

---

## 2026-05-20 — Teal cyan 180° theme

**Decision:** `[data-system="psx"]` ships `oklch(0.65 0.16 180)` —
teal cyan in the open 175-185° band.

**Why:** PS1 launch marketing palette was cool blue/cyan/silver (the
iconic "PlayStation" wordmark was a light blue/cyan gradient). Hue 180°
in the open 175-185° band sits 15° from Coleco bright cyan (195°),
40° from PCE-CD silver-cyan (220°), and 55° from segacd sapphire (235°).
No hue crowding. L=0.65 lifts it above the segacd L=0.55 / Saturn
L=0.45 "deep cool tones" cluster so the visual hierarchy reads clean
in a mixed library:

- PSX = cool bright cyan (open band, period-adjacent)
- Saturn = deep purple (cluster bottom)
- segacd = sapphire
- PCE-CD = silver-cyan

**Considered and rejected:**

- **Brand-accurate gray-blue ~210°.** Period-correct to PS1 launch but
  tight collision sandwich with Coleco 195° / PCE-CD 220° — would need
  very low chroma to read as distinct.
- **Yellow-green 110°** (Plan C from the question). Open band but
  PS1 had no green association.

---

## 2026-05-20 — CD extension disambiguation + .pbp as PSX-unique

**Decision:** Register `.cue / .chd / .iso / .m3u / .ccd / .toc` for
`psx` (same set PCE-CD / segacd / saturn claim — disambiguation via
per-folder Import Wizard rule) PLUS `.pbp` (PSP-format PS1 EBOOT
container, PSX-unique — no collision).

**Why:** `.pbp` is the PSP's PS1 EBOOT format, also used by PSone
Classics releases on PSN. Beetle PSX HW + SW both read `.pbp` directly.
No other libretro system in OA's lineup uses `.pbp`, so the extension
unambiguously identifies PSX content.

The standard CD container set collides with the other CD-shape systems
(PCE-CD / segacd / saturn) — disambiguated via per-folder Import
Wizard hint following the established pattern.

`.pbp` is added to `is_cd_extension` in `apps/oa-shell/src/main.rs` so
it triggers the same path-based load + PSX BIOS pre-check that other
CD images use. The container is single-file (no multi-track context
needed), but routing it through the CD path keeps the BIOS pre-check
firing consistently.

**Considered and rejected:**

- **Skip `.pbp` registration.** PSP-converted PS1 libraries are common
  enough (PSN PSone Classics, fan-converted EBOOTs) that excluding
  `.pbp` would force operators to remux to `.chd` before scanning.
  Not worth the operator friction.
- **`.pbp` as Bytes-source load (skip CD path).** Single file means
  Bytes-source would work, but the BIOS pre-check wouldn't fire and
  the operator would hit core-side BIOS-missing errors instead of
  OA's clean error toast. Worth the small overhead of treating
  `.pbp` as a CD container.

---

## 2026-05-20 — Six canonical BIOS SHA-1s

**Decision:** `PSX_BIOS_KNOWN_HASHES` ships with six entries spanning
JP / US / EU regions and v3.0 / v4.1 / v4.4 / v2.2 revisions.

**Why:** PSX shipped many BIOS revisions across its 1994-2004 retail
lifespan; six entries cover ~95% of operator-installed dumps.
Operators with rarer SCPH-9000 v4.5 or NetYaroze dev BIOSes get the
`OkUnknownHash` warn-level toast — the launch proceeds, and the
operator can validate against their dump's documented hash.

---

## 2026-05-20 — Z keyboard → Cross primary, breaking PSX physical layout

**Decision:** `default_psx_bindings()` follows the cross-system "Z is
primary on East pad" rule (keyboard Z → Cross / libretro B / East pad).
This intentionally breaks the period-correct PSX physical layout
(Cross on the south pad position, Circle on the east pad position
matches real DualShock hardware).

**Why:** OA's cross-system "primary action on East pad" convention
applies across every other console system (PCE/Lynx/NES/SNES/Genesis/
segacd/sega32x/Saturn). PSX-specific muscle memory (South=Cross
because that's where the bottom-of-diamond Cross button physically
sits) would break that consistency. Operators with strong PSX muscle
memory remap via the per-system Bindings dialog.

**Considered and rejected:**

- **Honor PSX physical layout (Cross=South, Circle=East).** Matches
  real DualShock muscle memory but breaks OA's cross-system "East =
  primary" convention. Defeated by the consistency argument — every
  other console pins primary to East; making PSX special creates a
  cross-system friction point.

---

## 2026-05-20 — DualShock analog sticks deferred to Phase 2

**Decision:** `default_psx_bindings()` ships the 14-button digital
DualPad layout. DualShock analog sticks (Left + Right) and L3/R3
stick clicks are deferred to Phase 2 alongside shared analog-input
infra.

**Why:** Same Phase 2 deferral pattern that Virtual Boy uses (right
D-pad via right analog stick), Intellivision uses (16-direction
disc), and Saturn uses (3D Pad analog stick). Once the shared
analog-input infra ships, all four systems light up simultaneously.

Most of the PSX library plays on the digital DualPad alone. The
notable exceptions:
- **Ape Escape** — the ONLY PSX game that requires DualShock to play
  at all. Unplayable until Phase 2.
- Tony Hawk 2/3/4 — playable on digital but designed for analog.
- Crash 3 — analog stick maps to subtle steering nuance; playable on
  digital.
- Metal Gear Solid — uses analog stick for stealth-walking speed
  modulation; playable on digital with reduced precision.

Document Ape Escape's hard analog dependency in `KNOWN_GAME_BUGS.md`
once Phase 1 operator validation surfaces it.
