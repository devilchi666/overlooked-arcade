# Theme-Builder Wishlist Audit — what the communities wanted but never got (2026-06-17)

**Purpose.** Second audit pass (sibling to
[MOTION_VOCABULARY_AUDIT_2026-06-17.md](MOTION_VOCABULARY_AUDIT_2026-06-17.md)).
The first pass cataloged what these frontends _can_ do; this one mines what their
theme authors **repeatedly asked for and never got** — the good ideas left on the
table — so OA can adopt the best of them deliberately.

> **Companion audit — read alongside this one.**
> [MOTION_VOCABULARY_AUDIT_2026-06-17.md](MOTION_VOCABULARY_AUDIT_2026-06-17.md) is the
> sibling pass: it derives the **motion** vocabulary to build (named effects + their
> parameter basis, locked as D57), where this doc covers the **broader theme features**
> we still want added. The two overlap only at motion — the Tier-1 **motion paths +
> keyframe-timeline** ask in §2/§3 here is the addition that feeds back into that doc's
> seed catalog. Treat the two as one "what to build" set: that for motion, this for
> everything else.

**Method.** Three parallel passes over feature-request boards + issue trackers +
forums: **BigBox/LaunchBox** (featurebase + forums + reddit), **ES-DE / HyperSpin
/ RetroArch** (GitLab/GitHub issues + forums + roadmaps), **Playnite / Pegasus +
cross-community** (GitHub issues + comparison threads). Items marked _inferred_
came from documentation/workaround patterns, not a single filed request.

---

## 1. Headline — the top community asks VALIDATE OA's locked direction

The loudest, most cross-cutting wishes are, almost one-for-one, decisions OA
already made. This is strong external confirmation we're building the right thing:

| Cross-community top ask | OA status |
| --- | --- |
| **"Kill XAML — give us a declarative CSS/HTML-style theme language for non-coders"** (BigBox's #1 request; Pegasus "too hard"; the entire reason the 3rd-party Theme Creator exists) | ✅ **Already our direction.** `.oatheme` = declarative TOML + tokens, **no author JS** (PD1); `DeclarativeShell` renders it. The locked low-floor/high-ceiling north star _is_ this ask. |
| **"A native theme settings/config UI — let a theme expose presets/toggles/fields"** (Playnite's single strongest signal; spawned the whole `ThemeOptions`/`ThemeModifier` plugin ecosystem) | ✅ **Already shipped.** `settings_schema` manifest field + S4 validator + S5.4 per-theme settings namespace + the Settings-IA declarative Appearance schema. Auto-generated UI from a declarative schema is exactly what they're missing and we have. |
| **"Real animation: keyframe timelines + custom easing + per-element/per-transition control"** (HyperSpin Tier-1 roadmap: `timeline`/`CustomEase`/`MotionPath`; ES-DE per-transition profiles; BigBox combined-animation crashes) | ✅ **Specced as the motion model (D57).** Shared keyframe/spring basis + per-event presets + author-tunable timing is precisely this. (One addition below: **motion paths**.) |
| **"Per-system / per-platform dedicated looks that auto-switch"** (Playnite per-platform themes; RetroArch per-console; HyperSpin per-system) | ✅ **Shipped (ARC 2).** Per-system layout/view + background/SFX tiers + palette substrate; themes opt in (D33/D34). |
| **"Modern GPU rendering, not bitmap/Flash layers"** (HyperSpin retiring SWF for native vector + shaders) | ✅ **Already our pipeline.** wgpu/WGSL; ARC 3 Thrust S adds theme-selectable game/bezel shaders. |
| **"Swappable complete looks as one installable package"** (RetroArch unified format / Ozone packs; theme distribution) | ✅ **Shipped channel.** `themes` pack type on the oa-packs content channel. |

**Takeaway:** OA is not playing catch-up on the headline asks — it's ahead on the
two biggest authoring complaints (declarative language + theme settings UI) and has
the motion model already specced. The value of this audit is the **gaps** below.

---

## 2. Good ideas never built — ranked by cross-community recurrence + fit

Each tagged with where it lands in OA and its current status.

### TIER 1 — adopt; high recurrence, clear fit

1. **Conditional / reactive theming — show/hide + swap bound to data and UI state.**
   _Universal gap._ ES-DE wants a boolean `NOT` operator + multi-step variant
   triggers + a "menu open" state to bind to; HyperSpin wants "show game-info only
   if no dedicated theme exists"; Playnite wants `<Condition>` bindings; BigBox has
   `DataTriggers` but "hard to author." Nobody has a clean declarative conditional
   layer. **OA lands:** the eventual home is Rhai (ARC 3 Thrust R), but a
   **declarative conditional subset** (visibility/variant bound to metadata + UI
   state, no scripting) could ship far earlier and covers ~80% of the asks. **Net-new
   for OA** beyond per-system layout switching.

2. **Dynamic color extracted from box art / art-reactive ambient backgrounds.**
   _Wanted everywhere, shipped nowhere_ — every community fakes it with hand-authored
   per-platform palettes (BigBox "Colorful" theme; OA's own S5.2 `SYSTEM_PALETTES` is
   the same hand-authored workaround). A true "ambient glow / background tint derived
   from the selected game's cover" is a **genuine differentiator nobody has.** **OA
   lands:** a platform capability feeding the motion `fanart-crossfade` /
   `ThemeBackground` tiers + a derived color token. **Net-new.**

3. **Live / hot reload of a theme while authoring.** The conspicuous absence _nobody
   even formally filed_ — Playnite is F8-restart, BigBox needs the app cycle, Pegasus
   too. **OA lands:** ARC 4 Theme Studio territory, but a cheap win is **watch +
   hot-reload `<exe_dir>/themes/community/<id>/` on disk change** for the declarative
   loader (we already have `cargo tauri dev` HMR for our own code; this extends it to
   end-user disk themes). **Net-new; cheap; large authoring-UX payoff.**

4. **Motion paths (bezier position curves) + per-element timeline authoring.**
   HyperSpin's Tier-1 `MotionPath` + `timeline` asks — the one motion capability NOT
   already in our D57 seed catalog (we have keyframes + spring + easing, but elements
   move along straight tweens). **OA lands:** an additive preset/param in the motion
   model — a `path` channel (bezier waypoints) + the keyframe-timeline escape hatch
   for power users. **Add to the motion seed catalog.**

### TIER 2 — strong ideas, schedule deliberately

5. **Rich composable list/grid rows (parent-child / relative item layout).** ES-DE's
   single biggest structural ask (#669): list items are one styled string; authors
   want image + text + badges in a per-item relative space. **OA lands:** extend the
   `list`/`grid` primitives to accept a composable item template. Pairs with the
   per-system layout substrate.

6. **Video backgrounds + boot/intro/attract sequences as built-ins** (not hacks).
   Cross-community (BigBox seamless cross-view video; Playnite PS5-style boot screens;
   HyperSpin per-game video). **OA lands:** **ARC 3 Thrust V** (already planned —
   `<video>` slots + attract tiers). This audit confirms the demand.

7. **Theme-bundled shaders + particle effects.** ES-DE #1404 (ship a GLSL shader with
   a theme); HyperSpin native particles; RetroArch procedural shader backgrounds.
   **OA lands:** **ARC 3 Thrust S** (theme-selectable shader presets) — extend to
   theme-_bundled_ WGSL + a particle primitive. Confirms Thrust S scope.

8. **Reusable components / theme inheritance / external override dir.** BigBox "no DRY,
   no inheritance — massive duplication"; ES-DE includes/variants nesting; ES-DE
   external `customizations/<theme>` dir so user edits survive updates. **OA lands:**
   manifest/loader ergonomics — component includes + a baseline-inherit tier (we
   already have `_baseline` asset cascades) + a user-override dir outside the theme
   folder.

9. **Responsive / multi-resolution + ultrawide/TV scaling.** HyperSpin authors hand-
   convert 4:3→16:9 and edit configs for 4K. **OA lands:** a layout concern — ensure
   primitives scale resolution-independently (we render in a WebView, so largely free,
   but worth an explicit pass + a themeable safe-area).

### TIER 3 — note for later / lower leverage

10. **More bindable data + documented bindings.** BigBox's "Most Played" locked inside
    a monolithic block + stale binding docs; Playnite "expand `{Name}` in all parts";
    ES-DE declarative custom collections; custom platform fields. **OA lesson:** keep
    the theme data API (`context_slots` / `THEME_CONTRACT.md`) **complete and
    documented** — the recurring pain is undiscoverable/locked bindings, not missing
    data. (Ties to the metadata-editor Wave-2 reference doc.)
11. **Theme distribution discovery — popularity/hot sorting, download counts.** Playnite
    #3734. **OA:** future gallery (deferred per DECISIONS G WAIT until user mass); the
    `themes` pack channel is the substrate.
12. **GIF / animated-image + media-from-URL.** Pegasus #1166/#1120. **OA:** GIF is cheap
    (WebView native); media-from-URL conflicts with the **no-network-from-theme** posture
    — defer/decline unless gated.

---

## 3. Concrete additions this audit feeds back

- **To the motion seed catalog (audit doc §3):** add **`path-move`** (bezier-waypoint
  position channel) + a **keyframe-timeline escape hatch** — the HyperSpin Tier-1
  `MotionPath`/`timeline` asks, additive to the D57 basis.
- **To the manifest/contract roadmap:** a **declarative conditional layer** (visibility/
  variant bound to metadata + UI state) as the pre-Rhai subset of Thrust R; a
  **color-from-art** capability token; **hot-reload of disk themes** for authoring.
- **Confirms existing plan scope:** Thrust V (video/attract) and Thrust S (shaders) are
  directly demanded; the declarative-first + theme-settings-schema + per-system bets are
  the community's top asks already in hand.

---

## 4. Sources

**BigBox/LaunchBox:** launchbox.featurebase.app (theming-for-bigbox-crossplat;
dynamic-playlist-theme-videos; custom-fields-for-platforms); forums.launchbox-app.com
(70773 CTC 2.5 eval; 37670 video transition selector; 74404 seamless cross-view video;
46721 binding paths / Most Played; 57186 transition timing; 51590 Colorful theme;
56574 wall-view bounty); feedback.launchbox.gg 9915075 (DataTriggers).
**ES-DE:** gitlab.com/es-de/emulationstation-de work_items 669 / 935 / 1354 / 1404 /
1596 / 1722 / 1736 / 1761 / 1833 / 2039 + THEMES.md / THEMES-DEV.md.
**HyperSpin:** bug.hyperspin-fe.com/roadmap; hyperspin-fe.com forums (27070 enhancement
req; 47524 layers & animations; 1011 conditional info; 44582 grouping/shaders; 33255
SWF issues; 4708 particles; 41142 HD video themes; 28972 16:9 conversion) + HyperTheme
animation-preview R72.
**RetroArch:** github.com/libretro/RetroArch issues 16480 / 19069 / 9242 / 10747 /
15537 / 10269 / 8721 / 18176; retroarch-assets 341; forums.libretro.com themes/ozone.
**Playnite:** github.com/JosefNemec/Playnite issues 3694 / 4244 / 3679 / 3569 / 3734 /
3787 + Themes wiki; ashpynov/ThemeOptions; Lacro59/playnite-thememodifier-plugin.
**Pegasus:** github.com/mmatyas/pegasus-frontend issues 1141 / 1142 / 1166 / 1120 /
1179 / 1175 / 1162 / 581 + docs example-simple-ch2.
**Cross-community:** steamdeckhq ES-DE 2.0; reddit r/launchbox / r/emulation /
r/playnite comparison threads.

_Verification: headline "already shipped/specced" mappings cross-checked against OA's
own substrate docs (manifest `settings_schema`, S5.4, ARC 2 per-system, PD1
declarative loader, D57 motion basis). GitHub/GitLab issue open/closed status is as of
2026-06-17; re-confirm if a specific item becomes load-bearing. Inferred items
(color-from-art demand, hot-reload, motion-path) are derived from workaround patterns
across multiple communities, not single filed requests — but the convergence is strong._
