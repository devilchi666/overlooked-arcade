# Per-System Custom UI — Plan

> **⚠️ ARCHITECTURE SUPERSEDED by Theming ARC 2 (2026-06-15, DECISIONS
> D32/D33/D34/D35).** This plan's *foundational stance* — per-system UI as "the
> DEFAULT OA experience" behind a **global master toggle**, with all character
> "hardcoded in-tree, no theme format" (§1, §2, §4) — is **retired**. Under the
> theming substrate: per-system UI is a **platform capability themes opt INTO**;
> per-system *content* (SFX/backgrounds/boot/Vectrex view) is **theme content**
> (Retroverse's), homed in the theme's asset tree, not a platform default.
> **Stage 1 Slices 1–5 (machinery) shipped + stand** (in `platform/`); **Stage 1
> pilots 6–9 + Stages 2–3 re-home into Theming ARC 2 as Retroverse content**
> (slice L6). The *content* specs here (pilot character, asset sourcing strategy,
> boot-animation policy) remain the brief for L6; the *modes / global-toggle*
> architecture is replaced. Go to
> **[theming-arc-2-per-system-layout.md](theming-arc-2-per-system-layout.md)**
> (§2 ownership line + §3 reconciliation table) for the current model.

**Status:** SUPERSEDED (see banner). Original: planning, locked after the 2026-05-25 advisor + operator planning session (second session of the day, separate from the guided-setup planning earlier).

**Author:** Operator + Claude (LLM pair).

**Owner-of-decisions:** the operator. This document records what was decided.

---

## 1. TL;DR

Make each system in OA feel like its own mini-experience — not just different colors, but different audio, different boot animations, different navigation feel, eventually different layout structure. **This is the DEFAULT OA experience**, not a power-user feature. Most operators see per-system custom UI without doing anything; an "off" toggle is available for the minority who want a uniform plain library.

Shipped in three stages, each a complete product on its own:
- **Stage 1 — Polish layer.** Per-system audio, boot animation, tile visual flourishes, background style. SystemUIConfig data model lands. 3 reference systems built fully (GB → NES → Vectrex). All other 37 systems get a tasteful baseline config so the whole library feels themed, not just the pilots.
- **Stage 2 — Behavior layer.** Per-system navigation (grid / carousel / list / wheel), per-system interaction feel (instant / delayed / physical), per-system tile emphasis (boxart / title / screenshot). Library view only; in-game UI stays uniform.
- **Stage 3 — Experience layer.** In-game overlays (pause menu, quick settings, save-state UI) themed per system. Transitions between library/game themed. Per-system information architecture (some systems surface different metadata).

The kiosk shell, planned separately, becomes the **theme editor for power users** who want to deviate from the built-in experiences. Kiosk consumes these built-ins as starting templates. Desktop normal users never touch the editor; they just experience the built-ins as the product.

Positioning: **"the only frontend where every system feels alive."**

---

## 2. Goals + non-goals

### Goals

- **Every system feels intentional.** Not generic library entries with a color swap. Each system communicates its identity through audio, motion, layout, presentation.
- **Default experience, not opt-in.** Most operators get this without configuring anything. First-launch defaults to ON.
- **Staged ship, each stage shippable.** Stage 1 alone is a real feature; Stage 2 adds depth; Stage 3 completes the vision. No half-finished intermediates.
- **Hardcoded brilliance.** All per-system character lives in OA's code/assets — no scripting, no user-creatable themes, no plugin format on the desktop normal version. Predictable, maintainable, low surface area.
- **Mission-aligned.** The "premium home for overlooked systems" positioning + the per-system care that already exists in OA both call for this. Per-system custom UI is the logical extension of the per-system theming that's been there since day 1.

### Non-goals

- **No theme editor on desktop normal.** Theme editing is a kiosk-shell feature, period. Desktop operators don't get a "Theme Studio." They get the choice of: Themed (built-in per-system experiences), No theme (uniform plain library), or eventually Kiosk (full editor).
- **No user-creatable themes on desktop.** No `.oatheme` archive format, no Rhai scripts, no community theme index for the desktop experience. The theme ecosystem (decided WAIT in 2026-05-25 DECISIONS entry G) stays parked. Theme authoring lives in kiosk shell when/if it ships.
- **No per-system emulator behavior change.** Per-system custom UI affects the SHELL — library view, navigation, audio, overlays. Emulator runtime (input dispatch, save states, rewind, etc.) stays uniform; only the wrapping experience changes.
- **No forced novelty.** Operators who hate it can turn it off (uniform plain library). Some operators just want a list of games; OA respects that.

---

## 3. Relationship to other plans

### Theme ecosystem WAIT lock (DECISIONS G, 2026-05-25) — unaffected

The WAIT lock prohibits **user-creatable themes** (Rhai-scripted, .oatheme archives, federated index). This plan does NOT introduce any of those. All per-system character is hardcoded in-tree by the project. WAIT lock stays.

### Kiosk shell plan (docs/features/kiosk-shell/KIOSK_PLAN.md) — relationship clarified

Pre-this-plan, the kiosk shell Phase 2 spec described a TOML + Rhai theme substrate as if it were the source of all theming. After this plan:

- **Kiosk shell becomes "the theme editor + power-user mode."** Operators who run kiosk mode can optionally edit themes (the existing Phase 2 plan) OR consume the built-in per-system experiences as their starting defaults.
- **Built-in per-system experiences become kiosk shell's default themes.** Phase 2's "3 reference themes (Showcase / Minimalist / Cabinet)" gets reframed as "the built-in per-system experiences PLUS optional user-authored themes on top."
- **Desktop normal users never see the kiosk theme editor.** The kiosk shell remains an opt-in mode for the smaller cabinet-builder + power-user audience.

This plan effectively REPLACES the "where does the desktop default UX come from?" portion of the kiosk shell plan. The kiosk shell still ships its theme substrate eventually, but as power-user authoring on top of the built-ins, not as the source.

### Guided-setup plan (docs/PLANS/guided-setup.md) — order question

Guided-setup is the next major arc (8-10 weeks, awaiting Phase 0 green-light to start controller-nav primitives). This per-system custom UI plan is also multi-stage and multi-month. They can't both be next.

**Decision deferred.** This plan describes WHAT per-system custom UI is and HOW it ships. The ORDER vs guided-setup is a separate scheduling decision — could be after guided-setup, could be parallel (if multiple sessions overlap), could be alongside.

One alignment: Stage 1 of per-system UI overlaps usefully with Phase 0 of guided-setup (the controller-nav primitives) — both need focus management, focus rings, on-screen hint bars. If they ship together, the infrastructure can be shared.

### Cross-system theming today (frontend/src/themes/registry.ts) — extended, not replaced

The current `systemThemes` map in `registry.ts` holds per-system CSS variables (accent color, default font, default shader preset, tile aspect). This stays. The new `SystemUIConfig` adds BEHAVIORAL fields alongside the existing visual fields. Two configurations side-by-side, or merged into one — implementation choice (see Open Questions §14).

---

## 4. The three modes

Three top-level user experiences. Operator picks via a Settings → Display toggle (+ kiosk shell's own mode toggle later).

### Mode 1: Themed (default)

Per-system custom UI as designed in this plan. First-launch defaults to this mode. No further setup required from the operator — every system in their library has built-in audio, animations, visual personality.

### Mode 2: No theme (uniform plain library)

Single unified library UI. No per-system audio, no boot animations, no tile flourishes. Universal click sound (or none). Universal grid layout. Universal background. Tiles still show cover art and titles; the visual chrome is plain.

This is the "I just want a library, no fancy stuff" mode. For operators who:
- Find per-system theming distracting
- Have an accessibility preference for reduced motion
- Want OA to look uniform across systems for personal taste
- Have very low-end hardware that struggles with animations

**How to enter:** Settings → Display → "Per-system experiences" toggle → OFF. Applies immediately; no restart needed.

### Mode 3: Kiosk (future, separate plan)

Full themable mode. Operator can:
- Use the built-in per-system experiences as starting defaults
- Author new themes via the kiosk Theme Studio
- Use NO theme at all and have a plain kiosk shell
- Import community themes (eventually, when theme ecosystem unlocks)

**How to enter:** Settings → Mode → Kiosk (only appears once kiosk shell ships). Not in scope of this plan. See `docs/features/kiosk-shell/KIOSK_PLAN.md`.

### Path matrix

```
                    │ Per-system audio │ Boot anim │ Per-system layout │ Theme editor │
────────────────────┼──────────────────┼───────────┼───────────────────┼──────────────┤
Themed (default)    │      ✓           │     ✓     │ Stage 2 onward    │      ✗       │
No theme            │      ✗           │     ✗     │      ✗            │      ✗       │
Kiosk (no theme)    │      ✗           │     ✗     │      ✗            │      ✓ (off) │
Kiosk (built-ins)   │      ✓           │     ✓     │ Stage 2 onward    │      ✓       │
Kiosk (custom)      │   author-defined │ author    │   author-defined  │      ✓       │
```

---

## 5. Architecture — hybrid (config + escape hatch)

**Default approach: SystemUIConfig data model.** Every system fills out a config object that drives shared rendering primitives. Adding a new system = filling out the config.

**Escape hatch: per-system Solid component.** A handful of "signature" systems (Vectrex confirmed; others TBD) override the config-driven library view with a custom Solid component when they need to do something the config DSL can't express. Vectrex's vector-stroke rendering is the canonical example — that's not a config value, it's a different render path.

**Default code path:**

```
SystemUIConfig per system
       ↓
LibraryView (shared component, reads config)
       ↓
Renders: tile grid + nav + flourishes + audio hooks + boot animation
```

**Escape hatch path:**

```
SystemUIConfig per system (still exists for the global toggle + non-overridden fields)
       ↓
Custom component lookup
       ↓
SystemSpecificLibraryView (e.g. VectrexLibraryView.tsx)
       ↓
Renders: whatever the custom component does, with shared primitives where useful
```

**Implications:**
- Most systems are config-driven. Cheap to add, consistent behavior.
- A custom component is a deliberate architectural escalation. Don't reach for it unless config can't express what the system needs.
- All custom components share the same external interface (receives library data + reactivity; provides nav callbacks). Internals are free.
- The "Per-system experiences OFF" mode bypasses both — falls back to a single uniform library renderer.

---

## 6. SystemUIConfig data model (Stage 1 surface)

The config shape for Stage 1. Subsequent stages add fields without breaking existing config entries (additive).

```typescript
interface SystemUIConfig {
  // --- ChatGPT's original 6 (visual + behavioral character) ---

  /// Library grid shape. Stage 1: `grid` for all systems. Stage 2 unlocks the rest.
  layout: "grid" | "carousel" | "list" | "wheel";

  /// Movement behavior between tiles. Stage 1: `snap` baseline. Stage 2 tunes.
  navigation: "free" | "snap" | "paged";

  /// What's prominent on each tile.
  emphasis: "boxart" | "title" | "screenshot";

  /// Background shape behind the library.
  background: "static" | "animated" | "shader";

  /// Audio character.
  audioProfile: "none" | "ambient" | "console";

  /// Selection / focus feel. Stage 1 ships `instant` baseline + `delayed`
  /// + `physical` for showcase pilots. "Physical" implementation TBD (likely
  /// tile-bounce + optional gamepad haptic via gilrs).
  interactionStyle: "instant" | "delayed" | "physical";

  // --- Added in this plan to reach pilot showcase quality ---

  /// Tile aspect. Overrides existing `systemThemes[id].tileAspect` if set.
  /// "auto" defers to the existing theme value.
  tileShape: "auto" | "square" | "portrait-3:4" | "landscape-4:3" | "wide-16:9" | "circle";

  /// Animation timing for nav + transitions. Maps to a duration multiplier.
  /// "instant" = no animation. "slow" = ~300-500ms. "cinematic" = ~500-1000ms.
  transitionTiming: "instant" | "fast" | "standard" | "slow" | "cinematic";

  /// Button-label convention for on-screen hint bar. Matters for Nintendo
  /// systems where A=B and B=A vs Xbox-convention.
  /// "auto" picks based on system family.
  buttonLabels: "auto" | "xbox" | "playstation" | "nintendo-handheld" | "nintendo-console" | "saturn" | "vectrex-custom";

  // --- Asset references (separate from style enums) ---

  /// Optional per-system background asset (image / video / shader path).
  /// Resolved against `<exe_dir>/assets/system-ui/<system>/backgrounds/`.
  /// null = use the `background` enum's default.
  backgroundAsset?: string;

  /// Optional per-system SFX bank. Each event maps to a sound file.
  /// All sounds live in `<exe_dir>/assets/system-ui/<system>/sounds/`.
  /// null = use the `audioProfile` enum's baseline.
  soundEffects?: Partial<{
    navigate: string;     // tile-to-tile cursor move
    select: string;       // tile picked
    back: string;         // exit / cancel
    launch: string;       // game starts loading
    bootIntro: string;    // boot-animation accompaniment
    bootOutro: string;    // exit-system accompaniment (Stage 3)
  }>;
}
```

**Stage 2 adds (planned):**

```typescript
  /// Layout-specific config for non-grid layouts.
  layoutConfig?: CarouselConfig | WheelConfig | ListConfig;

  /// Per-system focus-ring style override.
  focusRing?: "subtle-outline" | "bold-outline" | "glow" | "vector-stroke";

  /// Tile hover/focus animation (CSS keyframe set).
  tileBehavior?: "scale" | "tilt" | "glow" | "lift" | "vector-pulse";
```

**Stage 3 adds (planned):**

```typescript
  /// Per-system in-game overlay theming.
  inGameOverlayStyle?: "default" | "aggressive" | "minimal" | "vector";

  /// What metadata surfaces show on tile-focus popovers.
  metadataPriority?: ("title" | "year" | "developer" | "publisher" | "players" | "genre")[];

  /// Optional custom-component override for the entire library view.
  /// When set, the SystemUIConfig drives outer concerns (audio, toggle state)
  /// and the named component renders the library.
  customComponent?: "vectrex" | string;
```

---

## 7. Navigation pattern — flat grid + explicit system-entry (both)

Two coexisting library navigation paths. Operator can use either at will; the per-system experience is more pronounced in the entered state.

### Flat grid (existing behavior + enhanced)

The library grid that already exists. Sidebar can filter by system. As the cursor / focus moves across tiles:

- Tile-focus triggers **light per-system retheme** — accent color cycles, system-specific click sound on the next nav move, tile-focus animation reflects the system's `tileBehavior`. Background stays neutral (operator hasn't "entered" a system yet).
- No boot animation. No full audio profile.

This mode serves "I want to scroll my whole library."

### Explicit system-entry (new, when operator wants the full experience)

The Sidebar's manufacturer/system view, or a top-level "system selector" pattern, lets the operator deliberately enter a system:

- Click / select a system tile in the system selector OR pick a system from the Sidebar OR (Stage 3) launch from a "go to system" hotkey.
- Boot animation plays (~1-1.5s, skippable, see §10).
- Operator lands in the system's themed library — full background asset, full audio profile, system-specific layout (Stage 2+), full per-system character.
- Exit back to the library root → exit animation (Stage 3) → back to flat grid.

This mode serves "I want to be in the SNES section right now."

### Both coexisting

Operator can use either. The "Per-system experiences" toggle (Mode 2 = no theme) bypasses both — uniform plain library, sidebar filtering still works, no audio / no animation regardless of nav path.

### Why both

- Flat grid is the muscle-memory pattern existing operators expect; removing it would break their flow.
- System-entry is where the per-system character truly lands — boot animation, full audio, system-specific layout (Stage 2+).
- Both costs slightly more frontend complexity than either alone (need to handle two nav paths) but doesn't double the work — the underlying library rendering is shared.

---

## 8. Coverage — baseline for all systems + showcase pilots

### Pilot order: Game Boy → NES → Vectrex

Each pilot demonstrates a different point on the per-system character spectrum and validates the SystemUIConfig + escape-hatch architecture progressively.

#### Pilot 1: Game Boy (Stage 1)

**Why first:** smallest scope per stage. Establishes the "soft / minimal / personal" end of the spectrum. If the project goes sideways, GB alone is shippable as one well-themed system.

**Stage 1 deliverables:**
- `audioProfile: "ambient"` — soft click ("tap"), quiet back ("soft tone")
- `background: "static"` — single soft DMG-greenish gradient
- `interactionStyle: "delayed"` — slight ease-out on tile-focus (feels like LCD persistence)
- `tileShape: "portrait-3:4"` — handheld convention
- `transitionTiming: "fast"`
- `buttonLabels: "nintendo-handheld"` — A=B convention
- Custom boot animation: short LCD fade-in (~1s)
- `soundEffects` populated with 4-5 original or curated sounds

#### Pilot 2: NES (Stage 1)

**Why second:** validates the pattern at medium complexity. "Classic / bright / instant" — different point on the spectrum from GB, exercises the SystemUIConfig differently.

**Stage 1 deliverables:**
- `audioProfile: "console"` — toy-piano "boop" on nav, brighter click
- `background: "animated"` — subtle scrolling NES-palette pattern
- `interactionStyle: "instant"` — snap selection, no lag
- `tileShape: "auto"` (uses existing 4:3 default)
- `transitionTiming: "fast"`
- `buttonLabels: "nintendo-console"` — A=B convention
- Custom boot animation: quick zoom-in (~800ms) with palette flash
- `soundEffects` populated; bright/playful character

#### Pilot 3: Vectrex (Stage 1, custom component territory)

**Why third:** the swing-for-the-fences signature. ChatGPT said "this alone could go viral." Highest-risk-highest-reward. Done last because it likely needs the escape-hatch (`customComponent: "vectrex"`) and we want the config-driven pattern proven on GB + NES first.

**Stage 1 deliverables:**
- `customComponent: "vectrex"` — escape-hatch escalation; renders library tiles as stroked vector rectangles with phosphor glow shader
- `audioProfile: "console"` — synthesized vector-blip sounds (likely AI-generated or original)
- `background: "shader"` — phosphor-screen WGSL shader (low-intensity glow + scanline-blur)
- `interactionStyle: "physical"` — tile selection produces vector-pulse + brief glow bloom
- `tileShape: "square"` (override existing)
- `transitionTiming: "slow"` — cinematic feel
- `buttonLabels: "vectrex-custom"` — Vectrex's odd 4-button layout
- Custom boot animation: vector lines draw in (~1.5s)
- `soundEffects` fully custom (likely AI-generated synthesized blips)

### All other ~37 systems — baseline SystemUIConfig (Stage 1)

Every other system gets a `SystemUIConfig` with sensible defaults so the whole library feels themed, not just the 3 pilots:

```typescript
const BASELINE_CONFIG: SystemUIConfig = {
  layout: "grid",
  navigation: "snap",
  emphasis: "boxart",
  background: "static",       // soft system-accent gradient (uses existing systemThemes[id].accent)
  audioProfile: "ambient",    // universal soft click for nav + select
  interactionStyle: "instant",
  tileShape: "auto",
  transitionTiming: "fast",
  buttonLabels: "auto",       // resolved per system family
};
```

Each system can override any subset with a one-line patch in the `systemUIConfigs` registry. Most systems will need 0-2 overrides; the showcase pilots need many.

**Net effect:** the entire library has at least baseline per-system character (click sounds, accent-colored backgrounds, sensible tile flourishes) at Stage 1 ship. GB / NES / Vectrex stand out as the showcase tier. Stage 2+ progressively tunes more systems.

---

## 9. Audio asset sourcing — multi-source (CC0 pack + originals + AI)

Per-system SFX is real content production. Strategy: combine three sources depending on what works for which sound.

### Source A — Royalty-free / CC0 pack

Curated CC0 pack as the baseline for nav / select / back sounds across all systems. Sources: Freesound.org CC0 filter, BBC sound archive's CC license subset, Kenney.nl's CC0 game asset packs.

Used for:
- Baseline universal click (default for all 37 non-pilot systems)
- Soft / generic console-y nav sounds where no system-specific character is needed
- Filler when an original recording isn't available

### Source B — Original recordings

Operator records short SFX themselves (or commissions them) for the showcase pilots and any system that needs a signature sound.

Used for:
- GB's "tap" — needs a specific quiet handheld feel
- NES's "boop" — needs the toy-like character that CC0 packs don't quite capture
- Any later system where the operator wants a specific character

Investment: ~1-2 hours of recording + editing per pilot system.

### Source C — AI-generated

AI text-to-sound or procedural synthesis for sounds that are tedious to record AND not available in CC0 packs.

Used for:
- Vectrex synthesized vector-blips — hardware-accurate synthesized sounds are easier to generate procedurally than record
- Future systems like Virtual Boy (LED-projector specific sounds) where the hardware character is synthesized
- Filling out 4-5 variant sounds per system without recording each one

Caveats: AI-generated audio quality varies; needs taste-curation pass per output. License of generated audio depends on the tool used; pick CC0-friendly tools.

### Source D — Community-sourced (NOT used)

Mission-aligned but slow + uneven. Defers to the kiosk-shell theme substrate if/when it matures. For desktop normal version, all assets ship with OA — no community submission path.

### Asset location on disk

```
<exe_dir>/assets/system-ui/<system>/
  ├─ sounds/
  │   ├─ navigate.ogg
  │   ├─ select.ogg
  │   ├─ back.ogg
  │   ├─ launch.ogg
  │   ├─ boot-intro.ogg
  │   └─ boot-outro.ogg     (Stage 3)
  ├─ backgrounds/
  │   ├─ default.png        (static)
  │   ├─ animated.webm      (animated)
  │   └─ shader.wgsl        (shader)
  └─ boot-animation/
      ├─ keyframes.css       (CSS animation)
      └─ effects.wgsl        (optional shader-based)
```

Bundled with the installer. Per-system asset weight target: ≤500 KB sounds + ≤2 MB visuals per system. Total addition over 40 systems: ~100 MB worst case; likely far less for non-pilot systems using shared baseline assets.

### Audio mixer integration

Per-system SFX flows through the existing 4-bus mixer (shipped 2026-05-24 in media-taxonomy). Specifically the `ui-sounds` bus. Each system's SFX plays at the operator's configured `ui-sounds` volume; no per-system volume by default (deferred to Settings if requested).

---

## 10. Boot animation policy

**Length:** medium (~1-1.5s). Long enough to feel deliberate; short enough not to annoy on repeated entry.

**Frequency:** every system entry. Switching back to a system you just left still plays the boot animation. (If this proves annoying in playtest, downgrade to once-per-session.)

**Skippable:** always. Any input cancels mid-animation and lands the operator directly in the themed library. No "press-to-skip" hint needed once the operator learns it; the animation is short enough that not-skipping is fine for first-time entries.

### Implementation constraints

- **Animation must not block UI.** Library data loads + tiles populate UNDER the animation. When the animation ends, the operator can act immediately. No "loading" state visible after the animation finishes.
- **Reduced-motion respected.** A `prefers-reduced-motion: reduce` CSS media query (or a dedicated Settings toggle) short-circuits the animation to a 200ms fade. Accessibility floor.

### Settings surface

Two related toggles in Settings → Display:

- **"Per-system experiences"** — master toggle for the whole feature. Default ON.
- **"Boot animations"** — sub-toggle, only visible when per-system experiences ON. Default ON; disabling drops to a 200ms fade for all system entries.

---

## 11. Stage 1 — detailed deliverables

What ships in Stage 1, the polish layer.

### Code work

1. `SystemUIConfig` interface + `systemUIConfigs` registry (one entry per system, mostly baseline).
2. Custom-component lookup mechanism (Vectrex escape hatch).
3. Shared library renderer reading from `SystemUIConfig`.
4. Per-system SFX playback wired through the existing 4-bus mixer (`ui-sounds` bus).
5. Boot animation framework — CSS keyframes by default, WGSL shader path for shader-based animations.
6. Background renderer — static / animated / shader paths.
7. Tile flourish system — applies `tileBehavior` from config (Stage 2's fields ship as no-ops if read; safe to set them in config now).
8. Settings → Display → "Per-system experiences" toggle.
9. Settings → Display → "Boot animations" sub-toggle.
10. `prefers-reduced-motion` honoring.
11. The three pilot systems' full builds (GB / NES / Vectrex including Vectrex custom component).

### Content work

12. CC0 pack curation: select baseline sounds for all systems (nav / select / back).
13. Original recordings or AI-generated sounds for the 3 pilots' signature SFX banks.
14. Vectrex phosphor shader (WGSL).
15. GB / NES background assets.
16. Boot animation keyframes/shaders for each pilot.

### Doc work

17. Each system's `docs/cores/<id>/README.md` gets a "Per-system UI" section noting what config it has + any signature character.
18. New file `docs/features/per-system-ui/STAGE_1.md` capturing what shipped.

### Ship criteria — "Stage 1 done" means

- Toggle works. Default ON. Operator can disable to get plain uniform library.
- All 40 systems have a SystemUIConfig (baseline for 37, showcase for 3).
- All 40 systems play SFX from the SFX bank on nav / select / back / launch.
- All 40 systems show a background asset (gradient default; pilots have more).
- All 3 pilots have a full custom boot animation.
- Vectrex has its custom component live (not just config).
- `cargo test --workspace` green.
- Operator playtest: launches GB → boots into themed GB experience → can navigate and launch a game → exits back to library. Same for NES and Vectrex.

### Estimated effort

- **Architecture + infrastructure (1-8):** ~3-4 weeks frontend.
- **Pilot builds (11):** ~1.5-2 weeks (0.5-1 week per pilot).
- **Content production (12-16):** ~1-2 weeks parallel with code work.
- **Total Stage 1:** ~5-7 weeks.

---

## 12. Stage 2 — behavior layer (outline)

Once Stage 1 is in playtest, Stage 2 adds the behavior dimension. **All Stage 1 deliverables stay; Stage 2 builds on top.**

### What gets added

- Layout-specific renderers for `carousel`, `list`, `wheel`. Operator-configurable per system; pilots that benefit from non-grid layouts get them (e.g. Vectrex might use `list` instead of `grid`; an arcade system might use `wheel`).
- Per-system `navigation` behavior implementation — `paged` for arcade systems, `snap` default, `free` for systems with very small libraries.
- Per-system `interactionStyle` — `delayed` for handhelds (LCD-feel), `physical` for arcade / Vectrex.
- Per-system tile emphasis — `boxart` default, `screenshot` for arcade systems, `title` for text-heavy minimalist setups.
- `focusRing` style overrides per system.
- `tileBehavior` (scale / tilt / glow / lift / vector-pulse) per system.

### New systems tuned beyond baseline

Stage 2 picks 5-10 more systems and tunes them to "showcase tier":
- Jaguar (aggressive / arcade / neon — ChatGPT's marquee example)
- PS1 (calm / ambient / floating tiles / delayed selection)
- Saturn (similar to PS1; slightly more 90s industrial)
- One arcade system (MAME) — carousel layout with marquee art emphasis
- TG-16 (cult-classic feel — soft and considered)
- Arrived TBD by operator picks during Stage 2 planning.

### Estimated effort

~4-6 weeks. Mostly tuning + content production for the additional showcase systems.

---

## 13. Stage 3 — experience layer (outline)

The full vision. **Stages 1+2 stay; Stage 3 completes.**

### What gets added

- **In-game overlays themed per system.** Pause menu, quick-settings overlay, save-state slot picker, screenshot toast all carry the system's character. Jaguar's pause menu has aggressive neon; GB's has soft pixel-font transitions.
- **Library ↔ game transitions themed.** Launching a game from a themed library plays a per-system launch animation (Jaguar slams in, GB fades, Vectrex draws). Exit-to-library plays the reverse. Already-themed boot transition gets a matching exit transition.
- **Per-system information architecture differences.** Some systems surface different metadata. Arcade systems show coin-up counters / control-panel diagrams; CD-based systems surface disc-art prominently; handheld systems show battery icons if applicable (just for vibe). The `metadataPriority` config field lands.
- **Custom-component escape hatch expanded.** More systems can take this path if they need it (Virtual Boy? Atari Lynx?). Each becomes a Solid component override.
- **All ~40 systems tuned past baseline.** Every system has at least one signature touch by Stage 3 end. The library is genuinely a different experience per system.

### Estimated effort

~6-10 weeks. Most expensive stage due to in-game overlay work + per-system tuning at scale.

---

## 14. Open implementation questions

Don't need answers to plan. Need answers during the build.

1. **`interactionStyle: "physical"` — what does this actually mean in code?** Tile-bounce on click (CSS keyframe)? Gamepad haptic via gilrs (already wired in oa-input)? Screen shake (CSS transform on the library container)? Likely all three layered for the "physical" character; tune per system.
2. **`SystemUIConfig` storage — separate from `systemThemes` or merged?** Currently `frontend/src/themes/registry.ts` holds visual theme; the new config could live in a sibling `systemUIConfigs.ts` or extend the existing structure. Merge is cleaner long-term; separate is easier to roll out without touching existing per-system color work.
3. **Custom component contract — what shared primitives must it use?** Probably: focus manager (for controller nav), audio dispatcher (for sound effects), accent CSS variables, library data accessor. Define an interface and document it before the second custom component arrives.
4. **Boot animation skip semantics — what counts as "any input"?** Just A/B button press? Any pad input? Click anywhere? Probably: A confirms (skip to library), B cancels (skip + back), other inputs ignored mid-animation. Tune in playtest.
5. **Background asset performance budget.** Animated WebM backgrounds can be heavy; shader backgrounds run every frame. Each pilot needs a perf measurement on low-tier hardware before commit.
6. **Per-system audio volume — universal `ui-sounds` bus or per-system override?** Universal for Stage 1 (operator controls bus volume globally). Add per-system volume slider only if operators complain that Vectrex blips are too loud vs GB taps.
7. **Asset bundling vs download — ship with installer or download on first launch?** Stage 1 ships ALL assets bundled (avoids network during onboarding, matches the existing "non-commercial offline-first" mission). Total asset weight target: ≤100 MB additional install size.
8. **Boot animation per system-entry vs per-launch-vs-per-day frequency tuning.** Plan says every entry; playtest may reveal it's annoying when frequently switching. Easy adjustment.
9. **System-selector top-level pattern (the explicit "enter a system" path) — is it the existing Sidebar's manufacturer view, or a new "system carousel" surface?** Probably reuse Sidebar's manufacturer view for Stage 1; add dedicated system carousel as a Stage 2 nice-to-have.
10. **Vectrex custom component scope — does it own its own input handling, or does the shared focus manager still drive it?** Likely shared focus manager (consistency) + Vectrex-specific rendering. Document the contract in the component when it's written.

---

## 15. v2 / future additions

Documented now; not in Stage 1-3 scope.

- **Per-system in-game UI for the actual emulator output** — bezels / borders / overlays per system. Some libretro shaders already do this (LCD bezel for handhelds); a dedicated per-system frame surrounding the emulator render area would deepen the immersion.
- **System Mode "memory"** — remember which system the operator was last in; relaunch into that system's themed view directly. Convenience for operators who play one system extensively.
- **Per-system attract mode (when idle in a system's themed view).** Subtle screensaver-style effects after N minutes of inactivity. Vectrex draws vectors; PS1 cycles ambient backgrounds; Jaguar runs a loop of game trailers.
- **Operator-facing "what does this system feel like?" preview** in Settings → Display → Per-system experiences. Click a system name → see a 5-second sample of its boot animation + click sounds.
- **Per-game tile flourishes** (Stage 3+ extension). Highest-played game on a system gets a subtle "favorite" indicator; new imports glow briefly. Game-specific signals on top of per-system theming.

---

## 16. Related plans + dependencies

- **`docs/features/kiosk-shell/KIOSK_PLAN.md`** — kiosk shell becomes "theme editor for power users" that consumes these built-in per-system experiences as defaults. Phase 2 of kiosk shell (theme substrate) sits on top of this plan, not under it.
- **`docs/PLANS/guided-setup.md`** — guided-setup's Phase 0 (controller-nav primitives) and this plan's Stage 1 (focus / hint bar / shared primitives) overlap. If they ship in the same window, infrastructure is shared.
- **`frontend/src/themes/registry.ts`** — existing per-system theming. Stays; `SystemUIConfig` extends or sits beside (decision in Open Question §14).
- **`apps/oa-shell/src/audio_player.rs`** — 4-bus mixer shipped 2026-05-24. New per-system SFX flows through the `ui-sounds` bus; no new audio infrastructure needed.
- **`apps/oa-shell/src/light_gun_systems.rs`** — declarative system catalogue pattern. `systemUIConfigs` registry follows the same shape.
- **`crates/oa-render/`** — wgpu/WGSL renderer. Vectrex phosphor shader + any shader-based backgrounds land here; existing shader-preset hot-reload infrastructure reusable for per-system backgrounds.
- **`docs/cores/<id>/README.md` files (40 files)** — each gets a "Per-system UI" section in Stage 1 doc work item 17. Sets up the system-by-system tuning that happens through Stages 2-3.
- **`docs/PARKING_LOT.md` 2026-05-25 theme ecosystem entry** — stays WAIT. This plan doesn't unlock it. Theme authoring remains kiosk-shell-mode only.

---

## 17. What "ready to start" looks like

Before Stage 1 code is written:

- Confirm order vs guided-setup. The two arcs can pipeline (Stage 1 of this overlaps with Phase 0 of guided-setup) or sequence (one fully ships before the other starts). Operator decision.
- Confirm asset sourcing approach is acceptable — operator willing to record GB and NES signature sounds; AI tool selected for Vectrex synthesis; CC0 pack curated for baseline.
- Confirm the SystemUIConfig storage decision (Open Question §14.2 — separate file or merge with `systemThemes`).
- Update `docs/NEXT.md` to elevate per-system UI as a planned major arc alongside guided-setup.
- Optional: capture the strategic decisions from this planning session in `docs/DECISIONS.md` (audience clarity, mode separation, hybrid architecture, multi-source asset strategy).

None block writing this plan; they're the natural next steps after planning closes.
