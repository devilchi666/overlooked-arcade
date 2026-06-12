# BigBox Theming — Competitive Research & Strategy Report

**Date:** 2026-06-11
**Author:** Claude (4 parallel research agents → synthesis)
**Purpose:** Deep-dive on BigBox (LaunchBox big-screen mode) theming —
how it works, what the community loves/hates/wants, the visual-editor
landscape, and the cinematic media/motion axis — mapped onto OA's
theming-substrate arc (ARC 1 ✅ nearly done, ARC 2 shaders/behaviors,
ARC 3 Theme Studio).

This is a **research + strategy** doc, not a plan. Decisions it implies
go to `DECISIONS.md` / `PLANS/theming-substrate.md` when scheduled.
Every external claim is URL-cited in the agent appendices; the most
load-bearing ones are cited inline.

---

## 0. TL;DR — the five things that matter

1. **BigBox is a WPF/XAML app; themes are literally `UserControl` view
   files** bound MVVM-style to host view-models. High ceiling, brutal
   floor. **OA already made the better substrate bet** (declarative
   tokens + Solid + WGSL, no per-theme compiled code). Don't relitigate.

2. **The single most important BigBox pattern to match is its
   per-platform differentiation** — the "Nintendo view looks different
   from the Sega view" effect the operator asked about. It's achieved
   **three ways at once**: per-platform *theme* assignment (stored as
   platform metadata in `Platforms.xml`), per-platform *view* selection
   (user/author setting), and **filename-keyed asset resolution**
   (`Images\Platforms\<Name>\…`) so one view re-skins itself per system
   with zero extra view files. **OA's D19 currently scopes per-system
   theming as "Retroverse-only, not a substrate contract." This is the
   one prior decision the research most strongly pressures.** See §3.

3. **The community's #1 unmet desire is a free, first-party, *round-
   tripping* visual editor.** BigBox has none; the beloved third-party
   "Community Theme Creator" (~13.6K downloads) is partly paywalled and
   **cannot re-open hand-authored themes**. This is exactly the gap
   ARC 3 (Theme Studio) targets — and the round-trip problem is solvable
   only if we hold a *single declarative source of truth* from day one
   (§6). Our token/manifest substrate already does this.

4. **The community's #1 complaint is themes breaking on every update.**
   OA's versioned manifest + validator (Phase 3 S4) is the structural
   answer; we should treat the theme contract as a stability promise,
   not just a schema. See §5.2.

5. **Cinematic media/motion is BigBox's biggest "love" *and* its biggest
   performance liability** — the WPF "airspace" problem means video can't
   be composited under UI, video engine churned 3× in 2025, and the
   incumbents' fix for motion lag is *turning motion off*. **OA's
   wgpu/WGSL pipeline, where video is just a sampled texture, structurally
   dissolves the airspace problem** and is the right home for ARC 2's
   shader/motion work. See §4 + §7.

---

## 1. How BigBox theming actually works (architecture)

**Substrate:** WPF + XAML on .NET, Windows-only. A theme is a folder under
`…\LaunchBox\Themes\<name>\` containing `.xaml` view files (only the ones
you override — the rest fall back to the default theme), an `Images\` tree,
a `Videos\` tree, a Big Box Views config file, and optionally a `Plugins\`
subfolder of compiled DLLs. Views are WPF `UserControl`s whose root is a
`Canvas`/`Grid`, bound via **Caliburn.Micro** MVVM, using custom controls
from `Unbroken.LaunchBox.Wpf` (`FlowControl` = the 3D coverflow,
`TransitionPresenter` = the animated content host).

**"Theme" vs "View" is the central distinction.** A *theme* is the whole
package; a *view* is one XAML file = one screen/layout. The UI is divided
into **sections** (Game Discovery Center, Platforms, Platform Categories,
Filters, Games, System Menu/Options). Each section offers several numbered
**view variants** — e.g. four `PlatformWheel*` variants, multiple
`Wheel*` / `Wall*` / `Horizontal*` games views, `CoverFlow`,
`CoverFlowWithDetails`, `Text`. The user (or the theme's defaults) picks
which variant is active per section. A theme styles whichever variants it
wants; unstyled ones degrade to default. The complete view-file
enumeration (~54 XAML files across root/child/list-item/details/popup
families) is in the official "Big Box Views" article.

**Escape hatch = full compiled C#.** A theme can embed a custom WPF control
implementing `IBigBoxThemeElementPlugin` (callbacks `OnUp/OnDown/
OnSelectionChanged/OnEnter`), placed into the view tree via an XAML
namespace import. There is **no sandboxed scripting language** — extension
is full .NET against the host assemblies. This is why theme zips carry
DLLs and need the Windows "Unblock" step on install. **Contrast with OA's
planned ARC 2 model: sandboxed Rhai behaviors + WGSL shaders, not arbitrary
native code.** Ours is safer to distribute and the right call for a public
ecosystem.

**Data binding:** Caliburn.Micro against host view-models —
`SelectedGame.Title`, `SelectedGame.Platform`, `SelectedGame.StarRating`,
`SelectedPlatform.Name`, etc., plus media resolved by interpolated path.
Some aggregate stats (Total Games, Most Played) were historically *not*
exposed as discrete bindable properties — a real gap OA should not repeat.

**Startup / Shutdown / Pause / Attract** are a *separate, lighter XML
subsystem* under `\LaunchBox\StartupThemes\`, with the same per-platform-
by-filename override trick (`<platform name>.xml`). Worth noting because
OA's "launch ceremony" surface (theme territory) is the analog.

---

## 2. The view families, in OA terms

BigBox's section/variant taxonomy maps onto what OA already calls
primitives. The mapping is the useful part:

| BigBox view family | OA equivalent (current) | Status |
| --- | --- | --- |
| Vertical/Horizontal Wheel | `WheelNav` (reserved contract, S5.5) | contract reserved, not built |
| CoverFlow / CoverFlowWithDetails | `CarouselNav` (S5.5, CoverFlow dogfoods it) | ✅ shipped |
| Wall / Wall2-4 (grid) | `GridNav` primitive | ✅ shipped |
| Text / TextList | `ListNav` primitive | ✅ shipped |
| Game Details / TwoColumnGameDetails | route-level (theme-owned) | theme territory |
| Marquee views (2nd monitor) | — (kiosk/multi-monitor, deferred D20) | seam reserved |
| Filters / Platform Categories | route-level (theme-owned) | theme territory |
| Popups (achievements, keyboard, PIN…) | engine territory (mostly) | engine surface |

**Takeaway:** OA's primitive set already covers the four core display
shapes (list/grid/carousel + reserved wheel). The BigBox "numbered variant
per section, user-selectable" model is a UX pattern we have *not* adopted —
OA themes own their routes wholesale rather than exposing N swappable
variants per section. That's a deliberate, defensible difference (our
floor is "pick a theme," not "pick a view per section"), but **per-section
view-switching is a feature power users on BigBox value and OA could expose
later as a theme-author opt-in** (a theme declares multiple layouts for the
library route; user toggles). Park it; don't build it now.

---

## 3. Per-manufacturer / per-system / per-game views — the operator's core question

This is what the operator specifically asked to crack: *"different themes
(or views) that change when in different manufacturers, systems, games like
BigBox."* Here is exactly how BigBox does it and what OA should take.

### How BigBox does per-platform differentiation (three layers)

1. **Per-platform THEME assignment.** A *different theme folder entirely*
   can be bound to each platform. The platform→theme mapping is persisted
   as platform metadata (`\LaunchBox\Data\Platforms.xml`); playlists get
   their own theme assignment too. This is the "Nintendo home looks
   nothing like the Sega home" effect at full strength.

2. **Per-platform VIEW selection.** Within one theme, each platform can be
   assigned a different *active view* (Nintendo → horizontal wheel, Sega →
   coverflow). Set via *Manage Theme Specific Options → [Theme] → Views*.
   BigBox ships official tutorials titled literally "Change View Or Theme
   Per Platform."

3. **Filename-keyed asset resolution (the cheap, powerful one).** A single
   view file re-skins itself per platform by interpolating the platform
   name into asset paths:
   `…/Images/Platforms/{Platform}/Clear Logo/{Platform}.png`,
   `…/{Platform}/Fanart/{Platform}.jpg`, `Videos\Platforms\…`. One view,
   N platforms, zero extra view files. The same trick drives per-platform
   startup screens (`<platform name>.xml`).

**There is no true per-*game* theming** — game-level differentiation is
data binding (each game's own box art / logo / video / details render into
the active view), not a separate theme per game. Worth stating plainly so
we don't over-scope: "per-game views" in practice means *the view stays the
same; the data/media swaps per game.* OA already does this.

### Where OA stands today (and the one real tension)

OA's **DECISIONS D19** currently locks: *"per-system theming is
Retroverse-only, not a substrate contract."* The substrate gives us:

- **S5.1 — theme→platform asset cascade** (`resolve_background_asset` /
  `resolve_ui_sound` walk `theme/<system> → theme/_baseline →
  system/<system> → system/_baseline`). This is *exactly* BigBox's
  filename-keyed resolution (layer 3 above), already generalized into a
  proper cascade and already a substrate feature. **OA matches BigBox's
  best per-platform trick today.**
- **S5.2 — per-system palette substrate** (`SYSTEM_PALETTES` typed map +
  per-theme `perSystemTokens` scoped override). A theme can already say
  "NES → cyan accent, PSX → magenta" and have it beat the global baseline
  inside its mount. This covers per-system *recoloring* (layer between #2
  and #3 above).

**What OA does NOT have, that BigBox does:** per-platform *layout/view*
swapping as a first-class substrate contract (BigBox layers #1 and #2 — a
genuinely *different IA shape* per system, not just different colors/assets).
Today an OA theme would have to branch internally on `SystemId` to render a
different layout per system, with no engine support, and D19 explicitly
says the substrate won't formalize it.

**Recommendation (for operator decision, not for this session):** Keep D19
as-is through ARC 1 — it's correctly scoped for shipping. But flag for ARC 2
planning that **per-system layout variation should become a substrate
contract**, because (a) it's the headline thing the operator wants from the
BigBox comparison, (b) the asset + palette cascades already establish the
"resolve-by-active-system" plumbing, so extending it to *layout/primitive
choice* is an incremental seam, not a rebuild, and (c) it's what makes
"each system gets a polished, dedicated home" (the project's founding
pitch in CLAUDE.md) literally true at the layout level, not just the
palette level. The clean shape: a theme manifest declares a default layout
plus optional `per_system` layout overrides; the active-system signal
(already wired) selects which primitive + token set mounts. This is the
natural ARC 2 companion to behaviors/shaders.

---

## 4. Movement, layers, media — the cinematic axis (ARC 2 territory)

### Media slots — BigBox's taxonomy is the spec to match/exceed

BigBox exposes **~47 named image types** (the plugin API `ImageTypes`
class): Box (Front/Back/Spine/Full/3D), Cart (Front/Back/3D), Disc,
ClearLogo, Banner, a Fanart family, **five distinct screenshot slots**
(Gameplay/Title/Select/GameOver/HighScores), arcade-specific (Marquee/
Cabinet/ControlPanel/CircuitBoard), advertisement flyers, and storefront
art. Platform-level: Banner, Clear Logo, Default 3D Box/Cart, Device,
Fanart, plus a newer Media Packs system (Platform Clear Logos, Icons,
Badges, Controller Inputs).

ES-DE adds one idea BigBox lacks: **`miximage`** — a *generated composite*
(screenshot + marquee + box + physical media auto-laid-out). No LaunchBox
equivalent. Worth a parking-lot note as a content-generation feature.

**OA action:** when ARC 2's media binding lands, use BigBox's ImageTypes
list as the checklist for our media-slot vocabulary (we already have a
MediaDb; the per-game metadata lives there per our memory note). The five
screenshot sub-slots and the arcade-specific set are easy to under-scope.

### Layering / compositing

Two models in the wild:
- **Numeric per-element z-index** (ES-DE: explicit `zIndex`, documented
  defaults — image/video 30, text 40, carousel 50 — plus opacity, color
  multiply tint, and a built-in **reflection** system; **no blend modes,
  masks, or shaders anywhere**).
- **WPF visual tree** (BigBox/Playnite: `Panel.ZIndex` + document order,
  `Opacity`, and — because it's full WPF — `OpacityMask`/`Clip`/
  `RenderTransform`/shader effects are *technically* available but rarely
  surfaced in theme tutorials).

**Crucial finding:** *named blend modes do not exist in any declarative
theme layer in this entire space.* The one real precedent (additive blend)
lives in the **Mega Bezel shader**, applied to the *game*, not the UI. The
established pattern is: **z-index + alpha + color-multiply tint in the
declarative layer; named blend modes belong in the GPU shader stage.** This
is a gift to OA's architecture — a WGSL pipeline can unify game feed,
bezels, and reactive UI in one compositor with real blend modes, which
*no incumbent offers in the theme layer.* Strong ARC 2 differentiator.

### Motion / animation

- **BigBox (richest):** full WPF Storyboards + easing (CubicEase, BackEase,
  ElasticEase by inheritance), BigBox-specific transition classes
  (`Fade/Explosion/Flip/Rotate`), `CurveAmount` on the wheel, looping
  video. **Gotcha:** a multi-second Storyboard *blocks view switching*
  until it finishes — a real responsiveness bug to avoid.
- **ES-DE:** fixed keyword vocabulary (`instant`/`slide`/`fade`), some
  carousel motion props (`itemScale`, `itemRotation`, `itemStacking`),
  **no easing-function control** — much lower ceiling.
- **Universal gaps:** no first-class **parallax** primitive (only hand-
  composited z-index + transforms); no **particle/ambient-effect**
  primitive anywhere; no **ambilight / art-reactive ambient background**
  (mature in Plex/YouTube, absent from every emulator frontend surveyed).

**OA action:** ARC 2's motion layer (the `motion` token group is already
*reserved* in `tokens.ts` per S3) should ship: (1) easing as a first-class
authorable property, (2) a preset gallery *on top of* a real keyframe model
(the HyperTheme combo — see §6), (3) parallax-by-depth and (4) at least one
art-reactive ambient mode as a "wow" differentiator. Avoid the BigBox
blocking-storyboard bug: transitions must be interruptible.

### Performance — learn from BigBox's pain

- **The WPF "airspace" problem** is *the* reason LaunchBox couldn't use its
  most capable video engine: video "renders on top of all other UI" — you
  can't draw overlays above it. Their video engine churned **3× in 2025**
  (libVLC → WMP → FFmpeg), breaking complex video-grid themes each time.
  **OA's wgpu model, where video is a sampled texture in the same pipeline,
  eliminates airspace entirely.** This is a genuine structural win — call
  it out in ARC 2 planning.
- **Image caching / disk I/O — not GPU — is the most-cited stutter cause.**
  Scroll stutter on an RTX 3070 traced to a near-full drive; fix was SSD +
  pre-caching. Lesson: async image loading + a sane cache are worth more
  than GPU tricks for perceived smoothness.
- **VRR/G-Sync causes transition stutter** in *both* BigBox and Playnite
  (independent confirmation), fixed by forcing "Fixed Refresh." Lesson:
  **OA should control its own present/vsync mode** rather than inherit the
  desktop compositor's VRR behavior.
- **BigBox themes are NOT resolution-independent** — they target 16:9 and
  stretch on 21:9; ultrawide needs dedicated theme forks. **OA's relative/
  token-driven layout should be resolution-independent by design** so we
  never fork a theme per aspect ratio. Anchor/constraint-based layout (the
  Unreal "anchor medallion" idiom) is the model to steal for ARC 3.
- **Per-source audio gain is missing in BigBox** (can't lower startup-video
  volume in-app). Trivial for OA's 5-bus mixer (kiosk plan) to beat.

---

## 5. Community sentiment — loves, hates, wants

(Primary corpus: the LaunchBox official forums, where this community
actually congregates; Reddit is poorly indexed for this topic.)

### 5.1 What they LOVE (what keeps people on BigBox)
- **Cinematic per-platform video + music** in the menu system. This is the
  differentiator. Legendary themes — **CriticalZone** (looping video
  backgrounds), **Unified** (a HyperSpin-look port, 481 forum replies /
  ~498 platforms) — are loved precisely for multimedia depth. A direct
  comparison notes Playnite themes "don't support videos for the platform
  menu system," which is why BigBox feels premium.
- **The library/database backbone** is called "unrivaled" — people tolerate
  theming friction *because* the underlying library management is best-in-
  class. (Relevant: OA's Virtual Library arc is the analog; theming sits on
  top of it.)
- **Per-platform theming exists and is valued** (§3).

### 5.2 What they HATE (recurring, high-signal)
1. **Creating a theme is HARD** — the XAML/WPF barrier is the #1 complaint.
   Even experienced WPF devs hit missing assemblies + no design view + must
   reverse-engineer existing themes. *(OA's declarative-first floor + ARC 3
   visual editor is the direct counter.)*
2. **Themes break on every BigBox update** — top recurring grievance, a
   real maintenance tax. v11.10 changed Wall/Grid behavior and forced
   authors to re-tune across their whole catalog. *(OA's versioned manifest
   + validator is the structural answer — we should brand the theme
   contract as a stability promise. See §7.)*
3. **Performance** — lag even on high-end hardware; the standing fixes are
   to *disable the very features that make BigBox attractive* (backgrounds,
   coverflow, transitions). *(OA wgpu compositor + async caching, §4.)*
4. **The Wall/Grid view is architecturally limited** — the single most-
   cited "can't do it." Authors pay bounties / defect to Pegasus over it.
   *(OA's `GridNav` is a real primitive, not a bolted-on afterthought.)*

### 5.3 What they WANT (most-requested)
- A **better, fully-customizable grid/wall view** (#18 on the community
  feature poll; people post bounties for "console-like / PS-style grid"
  themes).
- **Easier theme creation / a visual editor** (the largest latent demand —
  see §6).
- **More per-platform control** (save individual platform views across
  themes; per-system recents/favorites).
- **Better/lighter video handling** (consistent engine, codec-free,
  performant).

### 5.4 Competitive read
- BigBox **wins** on multimedia richness, library DB, and theme ceiling.
- BigBox **loses** to **Pegasus** on grid/layout flexibility (authors
  defect), and to **Playnite** on fullscreen-mode + plugin ecosystem +
  ease.
- A chunk of BigBox's best themes (Unified) exist to *recreate HyperSpin's
  look* — HyperSpin remains the aesthetic reference for the wheel.

---

## 6. The visual editor — ARC 3 (Theme Studio) intelligence

**Headline:** across the *entire* emulator-frontend space, true WYSIWYG
theme editors are vanishingly rare. This is the biggest open opportunity.

### The landscape
- **BigBox:** no built-in editor. The community's answer is the third-party
  **Community Theme Creator** (y2guru) — a genuine WYSIWYG GUI that
  generates XAML, **~13.6K downloads, ~300K views**, reviewers building a
  full theme with *zero coding experience*. Enormous proven demand. **But:
  it's third-party, now partly Patreon-paywalled, and CANNOT re-open hand-
  authored themes** — only themes it created itself. This is the round-trip
  failure that defines the opportunity.
- **Playnite:** no built-in editor; points authors to **Microsoft Blend**
  ("go install Visual Studio").
- **ES-DE / Batocera:** XML + text editor + F5 reload. One community visual
  editor (ES-Theme-Editor) exists but is **unmaintained + Windows-only**.
- **Pegasus:** QML (programming); Qt's visual designer "couldn't be used
  because of crashes."
- **HyperSpin / HyperTheme:** the one strong WYSIWYG precedent — real-time
  drag-and-drop canvas, transform handles, grid overlays, layer grouping,
  unlimited undo/redo, **direct PSD import with layer preservation**, **63+
  animation types on a timeline with easing/loop/yoyo**, multi-aspect-ratio.
  **This is the closest existing model to OA's ARC 3 target — study it.**
- **GameEx:** shipped a (beta) first-party visual editor — rare proof it's
  doable in-house.

### The hard problem — round-tripping (this decides ARC 3's architecture)
Three boundary models exist; only the first round-trips power-user content:

- **Model A — code-is-truth, canvas is a live view of it** (round-trips
  losslessly *because the canvas has no separate persistence to reconcile*):
  SwiftUI + Xcode Previews; **Unity UI Builder** (visual edits ↔ UXML/USS
  text are two views of one document, with live preview panes — the closest
  markup analog to OA's goal); **Framer** code components via
  `addPropertyControls` (the schema is declared *in the source*); Builder.io
  `registerComponent` (Builder stores no code); **Figma Code Connect**
  (*maps* into named handles rather than regenerating).
- **Model B — declarative compiles one-way; escape hatch is an opaque,
  unmanaged region** (MJML `mj-raw`, Webflow custom-code embed). The visual
  tool can't introspect the hatch; author owns it.
- **Model C — two sources of truth / regeneration** (WordPress Gutenberg
  block-invalidation; Figma→code re-inference). **Inherently lossy. Avoid.**

**The load-bearing conclusion for ARC 3:** the systems that round-trip
power-user content cleanly all keep a **single declarative source of truth**
and treat the visual editor as either (1) a live *view* of that source, or
(2) a panel of controls whose *schema is declared in the source itself*, or
(3) a *mapping* into named handles — **never** a regenerated second
artifact. **OA's token/manifest substrate is already a single declarative
source of truth (Model A).** The ARC 3 mandate is therefore: the Theme
Studio reads and writes *named values into declared handles* (tokens,
manifest fields, primitive props, behavior params) and **never rewrites the
hand-authored body** of a custom primitive/behavior. Power users author
components/handles in the source; the visual editor edits the values, not
the bodies. This is the exact implementable form of the project's north-
star memory note ("declarative-first + escape hatch so the Theme Studio can
round-trip what power-users author").

### Editor design ideas worth stealing (for ARC 3 planning)
- **Layers/z-order = list order** with direct-manipulation transform
  handles in a live preview — the **OBS model** that non-technical
  streamers learned effortlessly. Keep *one* consistent "top of list =
  front" rule (Figma's auto-layout z-order inversion is a known footgun).
- **Data binding for non-coders = "design one, repeat many" + a field
  picker.** For "this text shows the game's release year," expose a
  *dropdown of game metadata fields*, never an expression box. (Webflow
  Collection fields / Framer Bind-to-CMS / Unity `dataSourcePath`.)
- **Responsive = anchors/constraints** (Unreal anchor medallion) over
  breakpoints, for a fixed-aspect TV UI — and resolution-independent so we
  never fork per aspect ratio.
- **Animation = preset gallery on top of a real keyframe timeline**
  (HyperTheme proves the low-floor/high-ceiling combo).
- **Live preview against *real* game metadata**, not lorem-ipsum. The
  F5-reload frontends feel primitive; this is table stakes.

---

## 7. What this means for OA — consolidated recommendations

Ordered by leverage. None are for *this* session — they feed ARC 2/3
planning and the operator's roadmap calls.

**A. Validate the substrate bets we already made (no action — confidence).**
The research strongly confirms: declarative-first + token source-of-truth
(vs XAML), wgpu compositor (vs WPF airspace), sandboxed behaviors (vs
compiled DLLs), versioned manifest (vs break-on-update), resolution-
independent layout (vs 16:9 forks), real `GridNav` primitive (vs BigBox's
weak Wall view). OA is structurally positioned to beat BigBox on its four
biggest complaints. Don't relitigate the pillars.

**B. Elevate per-system *layout* variation to a substrate contract in
ARC 2 (the operator's core ask).** §3: the asset cascade (S5.1) and palette
override (S5.2) already give per-system *media + color*. The missing piece
— and the headline BigBox parity item — is per-system *layout/primitive*
choice. Extend the existing "resolve-by-active-system" plumbing so a theme
manifest can declare `per_system` layout overrides. Revisit D19's
"Retroverse-only" scope at ARC 2 planning. **This is the single highest-
value follow-up from this research.**

**C. Treat the theme contract as a stability *promise*, not just a schema.**
Break-on-update is BigBox's #2 hate and an author-retention killer.
Document a versioned-contract stability policy alongside the validator
(deprecation windows, schema_version migration path). Cheap, high-trust.

**D. Use BigBox's `ImageTypes` list as the ARC 2 media-slot checklist.**
~47 image types incl. 5 screenshot sub-slots + arcade-specific set. Easy to
under-scope. Add ES-DE's generated `miximage` to PARKING_LOT as a content-
gen idea.

**E. Make ARC 2's compositor lean into what no incumbent has:** named blend
modes in the theme layer, parallax-by-depth, an art-reactive ambient
background, interruptible transitions (avoid BigBox's blocking-storyboard
bug), and per-source audio gain (the 5-bus mixer beats BigBox trivially).

**F. ARC 3 Theme Studio = Model A round-tripping, full stop.** Single
declarative source of truth; editor writes named values into declared
handles, never rewrites hand-authored bodies. Study Unity UI Builder
(markup round-trip) + HyperTheme (the domain's WYSIWYG precedent) + OBS
(layer UX) + Framer `addPropertyControls` (schema-in-source). This directly
beats the CTC's fatal "can't re-open hand-authored themes" flaw.

**G. Performance discipline from day one:** async image caching (the real
stutter cause), OA-controlled present/vsync (the VRR fix), resolution-
independence. Perceived smoothness is won at the I/O + present layer, not
the GPU.

---

## 8. Open questions for the operator — RESOLVED 2026-06-11 (see DECISIONS D32)

1. ~~**D19 revisit:** per-system *layout* variation as a substrate contract?~~
   **RESOLVED — YES.** Per-system layout becomes a first-class substrate
   contract in ARC 2 (theme-declared per-system defaults). Expands/supersedes
   D19. → DECISIONS **D32**.
2. ~~**Per-section view-switching:** adopt or park?~~ **RESOLVED — ADOPT, and
   END-USER-OVERRIDABLE.** A theme offers multiple view types (manufacturer/
   system/game) + per-view layout primitives, mix-and-match like BigBox; the
   **end user can override the active layout at runtime, persisted.** → D32.
3. ~~**ARC 3 sequencing:**~~ **RESOLVED — Theme Studio stays AFTER ARC 2.**
   You need the layout/motion/shader capabilities to exist before a visual
   editor for them is meaningful. → D32.

---

## Appendix — research provenance

Four parallel research agents (2026-06-11), each fully URL-cited in their
raw output (retained in session transcript):
- **Agent 1 — BigBox architecture:** view taxonomy, per-platform mechanism,
  plugin model. Primary sources: official Big Box Views article, Plugin API
  docs, real theme `.csproj` + view XAML on GitHub, LaunchBox forums.
- **Agent 2 — community sentiment:** loves/hates/wants, the visual-editor
  demand. Primary corpus: forums.launchbox-app.com (CriticalZone, Unified,
  performance, theme-breakage, CTC threads).
- **Agent 3 — visual-editor landscape:** HyperTheme, Unity UI Builder,
  Framer/Webflow/Figma round-tripping, OBS layer UX, the three boundary
  models.
- **Agent 4 — cinematic media/motion:** ImageTypes taxonomy, compositing
  models, motion vocabularies, the WPF airspace problem + 2025 video-engine
  churn, performance root-causes.

Sourcing caveats: Reddit was not directly fetchable (forums carry the
weight); a few HyperSpin/Webflow pages 403'd (corroborated via secondary
sources); the deepest BigBox spec (`Documentation.pdf`) ships only inside
the LaunchBox install and is not public.
