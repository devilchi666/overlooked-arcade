# PLAY NOW tab — Retroverse default theme

**Status:** Design sketch. No code.

**Author:** Operator + Claude (LLM pair).

**Owner-of-decisions:** the operator. This document records what was sketched.

**Reference:** [features/per-system-ui/assets/default-theme-mockup.png](../features/per-system-ui/assets/default-theme-mockup.png) (HOME) +
[features/per-system-ui/assets/library-default-mockup.png](../features/per-system-ui/assets/library-default-mockup.png)
(LIBRARY) + [settings-tab-retroverse.md](settings-tab-retroverse.md). This doc
is the PLAY NOW companion.

---

## 1. TL;DR

PLAY NOW is fundamentally different from LIBRARY. LIBRARY is
*"browse a list,"* PLAY NOW is *"I want to play right now —
just help me pick."* Closer to Spotify's **Made for you** or a
streaming-service home than a library grid.

Same three-pane shell shape as LIBRARY / SETTINGS for visual
consistency, but the center is reweighted heavily: a massive
hero card at top followed by 3–4 Netflix-style horizontal
rails. The hero comes with a **"why this is recommended"**
one-line reason — that's the differentiator vs LIBRARY.

The mood sidebar **reweights** the recommendations (it doesn't
just filter). PLAY NOW is the only top-toolbar tab whose
default action on entry is *"pick the hero,"* not *"show me a
list."*

---

## 2. Layout shape

```
┌───────────────────────────────────────────────────────────────────────────────────┐
│ ◤ RETROVERSE      HOME  LIBRARY  COLLECTIONS  ▣PLAY NOW▣  DISCOVER  SETTINGS  ⌕  │
│   EMULATION FRONTEND                                              09:47 PM · 👤  │
├───────────────────────────────────────────────────────────────────────────────────┤
│ ┌─ MOODS ──────────┐ ┌─ ▣ FOR YOU ──────────────────────┐ ┌─ FOCUS ────────────┐ │
│ │                  │ │                                  │ │  BURNOUT 3         │ │
│ │ ▣ For you      ◀ │ │ ╔══════════════════════════════╗ │ │  Takedown          │ │
│ │ ⏵ Continue       │ │ ║                              ║ │ │ ┌──────────────┐   │ │
│ │ ⚡ Quick         │ │ ║      BURNOUT 3 TAKEDOWN       ║ │ │ │ [cover art]  │   │ │
│ │ ⌛ Marathon      │ │ ║      ─────────────────────    ║ │ │ │              │   │ │
│ │ 👥 With a friend │ │ ║      PS2 · Racing             ║ │ │ └──────────────┘   │ │
│ │ ⚔ Challenge      │ │ ║                              ║ │ │                    │ │
│ │ ★ Nostalgia      │ │ ║  Picked up where you left off ║ │ │ ★★★★★ 4.6 · 512   │ │
│ │                  │ │ ║  8h 12m · Time Trial #4       ║ │ │                    │ │
│ │ ─ DAILY ─────    │ │ ║  Last played 3 days ago       ║ │ │ Criterion · EA     │ │
│ │ ⤴ Surprise me    │ │ ║                              ║ │ │ Sep 7 2004 · 1–2P  │ │
│ │ 🎲 Daily roulette│ │ ║      [ ▶ PLAY NOW ]          ║ │ │ Played: 8h 12m     │ │
│ │                  │ │ ║      [ ↻ Show another ]      ║ │ │                    │ │
│ │ ─────────────    │ │ ╚══════════════════════════════╝ │ │ The ultimate high- │ │
│ │ ⓘ Recommendations│ │                                  │ │ speed driver — big │ │
│ │   adapt to time  │ │ ─ CONTINUE WHERE YOU LEFT OFF ─  │ │ crashes, takedowns,│ │
│ │   of day + your  │ │ ┌──┐┌──┐┌──┐┌──┐┌──┐┌──┐  ▸    │ │ multi-route mayhem.│ │
│ │   recent play.   │ │ │  ││  ││  ││  ││  ││  │       │ │                    │ │
│ │                  │ │ │  ││  ││  ││  ││  ││  │       │ │ ─ SCREENSHOTS ─    │ │
│ │                  │ │ └──┘└──┘└──┘└──┘└──┘└──┘       │ │ ┌────┐┌────┐┌────┐ │ │
│ │                  │ │ W4-1  8m   ★4-2  Lv7  …         │ │ │ ▓▓ ││ ▓▓ ││ ▓▓ │ │ │
│ │                  │ │                                  │ │ └────┘└────┘└────┘ │ │
│ │                  │ │ ─ QUICK SESSIONS (< 30 min) ──   │ │                    │ │
│ │                  │ │ ┌──┐┌──┐┌──┐┌──┐┌──┐┌──┐  ▸    │ │ YOUR PROGRESS      │ │
│ │                  │ │ │  ││  ││  ││  ││  ││  │       │ │ Achievements 12/38 │ │
│ │                  │ │ │  ││  ││  ││  ││  ││  │       │ │ Last played May 18 │ │
│ │                  │ │ └──┘└──┘└──┘└──┘└──┘└──┘       │ │                    │ │
│ │                  │ │ 12m  18m  10m  8m  25m  15m     │ │ [▶ RESUME] [⋯ MORE]│ │
│ │                  │ │                                  │ │                    │ │
│ │                  │ │ ─ FRIDAY NIGHT — MULTIPLAYER ─   │ │                    │ │
│ │                  │ │ ┌──┐┌──┐┌──┐┌──┐┌──┐┌──┐  ▸    │ │                    │ │
│ │                  │ │ │  ││  ││  ││  ││  ││  │       │ │                    │ │
│ │                  │ │ │  ││  ││  ││  ││  ││  │       │ │                    │ │
│ │                  │ │ └──┘└──┘└──┘└──┘└──┘└──┘       │ │                    │ │
│ │                  │ │ 2P   4P   2P   2P   4P   2P     │ │                    │ │
│ └──────────────────┘ └──────────────────────────────────┘ └────────────────────┘ │
├───────────────────────────────────────────────────────────────────────────────────┤
│  Ⓐ PLAY   Ⓑ BACK   Ⓧ SURPRISE ME   Ⓨ REROLL HERO   L1/R1 SWITCH RAIL   RS MOOD │
└───────────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Center pane — hero + rails

### The hero card

Takes ~40% of center pane height intentionally. The **"WHY" line**
is the differentiator from LIBRARY: every recommendation comes
with a one-line reason. Examples:

- `Picked up where you left off · 8h 12m in · Time Trial #4`
- `Short session friendly · ~12 min runs · perfect for your break`
- `From your favorites you haven't touched in 3 months`
- `Daily random pick · rerolls in 4h 32m`
- `2P couch co-op · friend over tonight?`

Removes the cognitive load of *"wait, why this one?"* instantly.

Two CTAs only:
- **PLAY NOW** — launches the hero
- **Show another** — rerolls the hero without launching

No third action; PLAY NOW is intent-focused, not a browse surface.

### Rails (below the hero)

3–4 horizontal carousels. Each rail has a clear angle and a
contextual reason chip per card:

- **CONTINUE WHERE YOU LEFT OFF** — save-state-equipped games,
  sorted by recency. Reason chip = save-state location ("W4-1",
  "Lv7", "★4-2").
- **QUICK SESSIONS** (< 30 min) — short-session games. Reason
  chip = typical session length.
- **FRIDAY NIGHT — MULTIPLAYER** — couch-co-op / versus.
  Time-of-day-aware label: becomes "SATURDAY MORNING — KIDS'
  PICKS" on Saturday AM, "MARATHON SUNDAY" on Sunday PM, etc.
- **HIDDEN GEMS** (offscreen below the fold) — high-rated games
  barely touched.
- **DAILY ROULETTE** — single card with a reroll button, locks
  once per 24h.

Rail cards are smaller than LIBRARY tiles. Cover + system glyph
+ small reason chip below — the chip changes per rail because
it's the **reason this card is here**, not generic metadata.

---

## 4. Sidebar — MOODS (left pane)

8 entries plus a small Daily group. Picking a mood **doesn't
just filter — it reweights** the hero + rails:

- **For you** (default, algorithmic blend)
- **Continue** (save-state only)
- **Quick** (short sessions)
- **Marathon** (long sessions)
- **With a friend** (2P+ required)
- **Challenge** (high-difficulty / not-yet-cleared)
- **Nostalgia** (least-recently-played from favorites)

Plus the Daily group:

- **Surprise me** — sidebar entry that **launches immediately**
  rather than re-rendering. Different from `Ⓧ SURPRISE ME` in
  the hint bar only in entry path; same behavior.
- **Daily roulette** — one card, reroll locked once per 24h.

Examples of reweighting:
- `MARATHON` raises long-session titles to the hero, reorders
  rails (QUICK rail collapses to a row of 3 cards instead of 6).
- `WITH A FRIEND` makes the hero a 2P+ pick and surfaces the
  multiplayer rail first.

---

## 5. Right pane — focused-card detail

Identical shape to LIBRARY's right pane: cover hero, chips,
description, screenshots, progress block, PLAY GAME / MORE.

- Auto-focuses on the hero card by default.
- Updates as the operator DPads through rail cards.
- Keeps the shell coherent — same component, different data
  source upstream.

For game cards not currently in the operator's recent rotation,
the progress block shows "Never played" instead of the usual
last-played / achievement stats.

---

## 6. Footer hint bar

- `Ⓐ PLAY` — launches whatever's focused (hero or rail card).
- `Ⓑ BACK` — backs out to HOME.
- `Ⓧ SURPRISE ME` — instant launch, skips detail panel.
- `Ⓨ REROLL HERO` — picks a new hero without launching.
- `L1 / R1 SWITCH RAIL` — moves focus up/down between rails.
- `RS MOOD` — quick-cycle through moods without leaving rails.

`Ⓧ` vs `Ⓨ` are intentionally distinct:
- **Ⓧ Surprise Me** — *"I trust the algorithm, just go."*
- **Ⓨ Reroll Hero** — *"this hero isn't it, give me a different pitch."*

---

## 7. Notable deltas vs LIBRARY / HOME

- **No grid, no system filter.** PLAY NOW isn't a browse surface
  — it's a recommendation surface. Browsing lives in LIBRARY.
- **Hero card with a "why" line.** The recommendation engine has
  to articulate its reasoning. Generic "here's a game" isn't
  enough.
- **Rails reweight per mood.** Rail set itself is curated; mood
  changes which rails appear and in what order.
- **Time-of-day labels.** Rails carry context-aware names
  ("Friday Night," "Saturday Morning") rather than generic
  categories. Small touch but makes the page feel alive.
- **Two reroll levels.** Surprise Me (launches) + Reroll Hero
  (re-picks). LIBRARY has neither.
- **Daily-cadence Daily Roulette.** Locks once you've used it
  for the day — gives operators a "ritual" hook without
  overwhelming.

---

## 8. Behavior model

PLAY NOW is the only top-toolbar tab whose **default action on
entry is "pick the hero,"** not "show me a list." The mood
sidebar is secondary nav, the right pane is contextual detail.

If the operator wants to *browse*, they're in the wrong tab —
that's LIBRARY. If they want to *explore* by system, that's
HOME. If they want curated lists, that's COLLECTIONS. PLAY NOW
exists for the single intent: *"I want to play, help me pick
fast."*

---

## 9. Implementation sketch (not committed)

Not a green-lit implementation plan — rough mapping in case it
ever ships:

- New `PlayNowPage` route at `frontend/src/routes/PlayNow.tsx`.
- A `RecommendationEngine` service computes the hero + rail
  contents from library DB + save-state inventory + play-time
  history. Pure function over the existing data; no new schema.
- Mood selection feeds into the engine as a weighting vector;
  the engine returns a `{ hero, rails: Rail[] }` object.
- Rails reuse a generic `Rail<Card>` component; cards are a
  variant of `LibraryTile` sized smaller with a reason-chip
  slot.
- Hero card is a new `HeroRecommendation` component — only
  used here.
- Right pane reuses the LIBRARY focused-card detail component
  unchanged.
- Time-of-day-aware rail labels live in a small lookup table
  keyed by `dayOfWeek + hourBucket`.
- Daily Roulette state stored in a small `daily.json` blob in
  the data dir; sentinel-guarded against multi-instance writes.

Status: idea, not in `ACTIVE_WORK.md`. Implementation will follow
once the operator green-lights the design and decides where this
lands relative to the existing per-system-ui Stage 2 arc.

---

## 10. Open questions for future planning passes

- **Recommendation source of truth.** Hero "why" lines need real
  data — play-time tracking per game (already a per-system-ui
  ASSETS.md note), save-state location parsing (per-core, may
  not be feasible everywhere), session-length averages
  (calculable from launch/exit timestamps).
- **Time-of-day awareness opt-out.** Some operators may find
  "Friday Night" labels too cute; needs a Settings → Per-system
  UI → Discovery-style "Plain labels" toggle.
- **Reroll fatigue.** What happens if the operator rerolls 20
  times? Cap at N rerolls per session, then offer to take them
  to LIBRARY instead.
- **Cold-start problem.** First-launch operator with no play
  history → engine has nothing to weight. Fallback: surface
  the operator's stated favorite systems from import wizard +
  highest-rated games per system.
