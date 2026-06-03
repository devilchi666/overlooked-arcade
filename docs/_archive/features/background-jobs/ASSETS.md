# Background Jobs — Operator-supplied assets

Where to drop the optional asset files the BackgroundJobs arc
expects. Every entry here is **optional** — the system silently
falls back when an asset is missing so OA doesn't go broken just
because the operator hasn't sourced one yet.

## Completion chime

**Where:** `<exe_dir>/assets/oa-ui/sounds/job-complete.<ext>`

**What:** A short subtle chime (~0.3-1.0 s, low loudness) played
through the `ui-sounds` audio bus when a job successfully
completes. Plan §"Notification on completion" — the chime carries
the "something finished, check the bar if you care" semantic; no
toast pops, no banner appears. The bar slides out + the chime
sounds.

**Extensions checked, in priority order:** `ogg`, `opus`, `wav`,
`mp3`, `flac`, `m4a`. The resolver picks the first one that exists.
Stick to one format — there's no fallback logic between them once
the first one matches.

**Naming:** Exactly `job-complete.<ext>`. Case-sensitive on Linux;
case-insensitive on Windows + macOS in practice but match the
lowercase exactly for portability.

**Failure mode:** No file present → completion fires silently. The
bar still updates, the recent activity panel still records the
finished row; just no audio. This is intentional — the chime is
nice-to-have polish, not a load-bearing UX signal.

**Toggle:** Settings → Background Jobs → Bar behavior → "Sound on
completion." Default ON. Operators who find the chime annoying
can toggle it OFF here even when the asset is present.

**Sourcing tips:** Anything that sounds like "operation done" works
— a soft single-note bell, a gentle "ping," a UI-style two-tone
acknowledgment. Avoid anything that sounds like an alert, alarm,
or error. Audio should peak at -12 dBFS or quieter so it sits
beneath whatever else is happening (platform music, game audio
when the operator is mid-play).

**Failure-event chime:** **Not currently implemented.** Plan
§"Notification on completion" specifies completion chime only;
failed jobs surface visually through the bar's red state pill and
the Recent Activity → Failed tab. Adding a distinct
`job-failed.<ext>` variant would be a separate small PR.

**Per-kind chime variants:** **Deferred to PARKING_LOT.md** per
plan §"Notification on completion" — would let core_download have
a distinct chime from artwork_sync. Useful eventually for
operators who multitask while OA works in the background; not in
scope today.

---

## Future asset slots (placeholders)

Reserved namespace for assets the arc may add later. None of these
exist as resolver paths yet — listed here so the layout stays
organized when they do.

- `<exe_dir>/assets/oa-ui/sounds/job-failed.<ext>` — distinct
  failure chime if/when added.
- `<exe_dir>/assets/oa-ui/sounds/job-start.<ext>` — optional
  acknowledgment chime when a job kicks off. Plan deliberately
  doesn't ship this (Toast Spam Avoidance principle).
- `<exe_dir>/assets/oa-ui/sounds/<kind>/job-complete.<ext>` —
  per-kind chime variants. PARKING_LOT today.

The `<exe_dir>/assets/oa-ui/` folder itself is OA-wide UI assets
distinct from `<exe_dir>/assets/system-ui/<systemId>/` (per-system
UI sounds — handled by the Per-System UI arc, not this one).
