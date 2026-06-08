# Audio Quality — SESSION_LOG

Cross-cutting audio-pipeline correctness work (collection → drain → resample
→ ring → cpal callback). Distinct from `audio_player.rs` (the rodio file
player for platform music / UI sounds / ceremony / snap — NOT in the live
game-audio path).

---

## 2026-06-08 — Sample-rate-feed bug (cross-system crackle + wrong pitch)

**Shipped:** Root-caused and fixed the "NES clipping + clicking" report (and
the broader "other systems too" suspicion). It was never amplitude clipping —
the shell never adopted the core's real sample rate after `retro_load_game`.
`LibretroCore::new` (`core.rs:192`) seeds a placeholder `Timing` of 44100 Hz /
60.000 fps because most cores can't report real `av_info` until a ROM is
loaded. `finish_load` snapshots the true values into the core, but the shell's
local `timing` and the `oa-audio` sink were constructed from the placeholder
and never refreshed. So the linear resampler was told `source_rate = 44100`
for every core regardless of reality → wrong output sample count + wrong
pitch:
- fceumm (real 48000): over-produced ~8.8% → ring overflow → dropped chunks
  (glitch) + high pitch.
- snes9x (real 32040): under-produced ~27% → ring underrun → crackle + low
  pitch.
- mednafen_pce (44100): fine only because real rate == placeholder.
- parallel_n64: fine only because it calls `SET_SYSTEM_AV_INFO` (env 32) — the
  one path that already rebuilt the sink at the real rate (commit `ed1e463`).

Diagnosis was log-driven, not theory: a 48 kHz stereo device drains
96000 i16/s; the per-120-frame `audio {pushed}+{dropped}` stat gave
Δpushed/s = 96.8k+drops (NES), 69.6k (SNES), 96.3k clean (PCE, N64) — the
off-by exactly matched each core's rate ratio.

Fix (commit `0bb4e89`, branch `feat/audio-quality`): after `load_rom` succeeds
in BOTH the runtime `EmuCommand::LoadRom` handler and the cold-start
direct-launch path, refresh `timing = core.timing()`, rebuild the audio sink
at the real rate, and retime the frame limiter when the rate/fps differ.
Mirrors the existing env-32 revision block; env 32 remains the secondary path
for cores that revise even later. Startup path now builds the sink AFTER the
ROM load (opens at the real rate directly, no build-then-rebuild). `oa-audio`
tests pass; operator playtest: "sounds much better."

**Almost:** Nothing partial — the rate fix is complete.

**Next:**
- (Operator, later) "Damn accurate" verification: confirm pitch/timing is
  exact across the full lineup, not just "better."
- IF a core still sounds genuinely hot after the rate fix, add a master
  soft-limiter in `oa-audio` (deliberately NOT added now — the data pointed to
  rate, and a preemptive limiter would be a guess). Re-open the NEXT.md MEDIUM
  entry only if a true amplitude clip is observed.
- Broader theme the operator flagged: we keep discovering libretro
  data/env callbacks the shell passes/handles incompletely (this bug; the
  env-32, env-63, controller-info, input-descriptor fixes before it). A
  dedicated libretro-plumbing audit is queued as the next arc.
