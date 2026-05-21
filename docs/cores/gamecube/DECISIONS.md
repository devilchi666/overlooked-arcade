# gamecube Decisions Log

Append-only. Newest at the bottom.

---

## 2026-05-20 — Dolphin as default

**Decision:** `dolphin_libretro.dll`. No practical alternate.

**Why:** Dolphin is the canonical libretro GameCube + Wii emulator
core. No competing libretro builds; standalone Dolphin (the
non-libretro version) is the most accurate but is harder to integrate.

---

## 2026-05-20 — Single slug covers GC + Wii

**Decision:** `gamecube` SystemId covers both Nintendo GameCube and
Nintendo Wii via Dolphin's runtime auto-detect from disc container.
Wii Remote / motion-controls deferred to Phase 2.5.

**Why:** Operator chose "Pair n64 + gamecube" (single GC slug)
during onboarding rather than the "Triple-pair n64 + gamecube + wii"
option that would split GC/Wii into separate slugs. Tradeoff
accepted: cleaner Phase 0 (one slug, one .dll, one set of bindings)
in exchange for mixed GC/Wii library shelf at Phase 0; Phase 2.5
polish may split if needed once Wii Remote dispatch lands.

**Considered and rejected:**
- **Triple-pair n64 + gamecube + wii (split GC/Wii).** Cleaner long-
  term UX (Wii Remote needs its own controller scheme) but adds an
  extra system's worth of plumbing without immediate value (no
  motion-control dispatch yet).
- **Single slug ignoring Wii entirely.** Would mean Wii ISOs don't
  classify; operators with mixed libraries would have to manually
  filter. Worse than auto-detect.

---

## 2026-05-20 — Analog input infra shipped alongside n64

**Decision:** The cross-cutting analog input infrastructure shipped
in this Phase 0 (see `docs/cores/n64/DECISIONS.md`) covers GameCube
too. Gamepad LeftStick → main stick (`axes[0..2]`), RightStick →
C-stick (`axes[2..4]`).

**Why:** GameCube games rely on dual analog sticks even more than N64
games rely on single — Smash Bros. Melee's C-stick smash attacks,
Metroid Prime's free-aim, Resident Evil 4's analog L/R triggers all
need the analog path. Shipping GC without analog support would force
operators into permanent core-option workarounds.

---

## 2026-05-20 — Indigo 280° theme (Nintendo home cluster)

**Decision:** `[data-system="gamecube"]` ships `oklch(0.48 0.22 280)` —
deep Indigo GameCube launch color in the violet cluster between
Saturn 275° (L=0.45 deepest) and GBA 285° (L=0.55 deep indigo).

**Why:** Period-correct to the iconic 2001 Indigo GameCube shell — the
default purple-blue plastic that became the platform's visual
shorthand. Forms the Nintendo home-console cluster with SNES 270° /
n64 268° / gamecube 280° / GBA 285°, four systems clustered in a 17°
hue range with a clear lightness ladder.

---

## 2026-05-20 — 12 digital buttons; C-stick is analog-only

**Decision:** `bindings::gamecube` ships 12 digital entries (d-pad +
A/B/X/Y + L/R + Z + START). The C-stick is NOT in the digital
bit-table; it flows through `InputState.axes[2..4]` exclusively
(gamepad RightStick).

**Why:** Unlike N64's C-buttons (which are 4 discrete digital
buttons on real hardware), the GameCube C-stick is genuinely analog
on real hardware — a smaller analog stick to the right of the
main stick. Treating it as 4 digital directions would discard the
analog precision that GC fighters / shooters / racers rely on (Smash
Bros. Melee's smash-attack momentum, RE4's free-aim, Mario Kart's
power-slide).

Keyboard-only users lose C-stick access at Phase 0 — there's no
keyboard fallback for the analog right stick. Phase 2.5 polish adds
per-axis keyboard binding (e.g. WASD → main stick, IJKL → C-stick)
so keyboard play becomes viable for GC.

**Considered and rejected:**
- **Map C-stick to 4 digital directions** (libretro L2/R2/L3/R3 +
  SELECT). Would let keyboard users access C-stick but discards
  analog precision. Defeated — most GC games are gamepad-first
  experiences where analog is the right model.

---

## 2026-05-20 — L/R triggers digital at Phase 0

**Decision:** L and R analog triggers map to digital libretro L (10)
and R (11) bits at Phase 0. Dolphin synthesizes analog pressure from
digital press.

**Why:** Real GC pads have pressure-sensitive L+R triggers (RE4
famously uses them for incremental aim-down). Dolphin's libretro
mapping supports analog triggers via the RETRO_DEVICE_INDEX_ANALOG_BUTTON
device class — which OA's `cb_input_state` returns 0 for at Phase 0
(deferred to Phase 2.5).

For Phase 0, digital trigger press is sufficient for the majority of
GC titles; the analog-trigger-sensitive minority (RE4) gets degraded
behavior until Phase 2.5 polish.
