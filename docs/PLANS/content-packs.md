# Content packs — distribution + update mechanism

**Status:** Design sketch. No code. Decisions locked 2026-05-28.

**Author:** Operator + Claude (LLM pair).

**Owner-of-decisions:** the operator. This document records what was decided.

**Reference:** Cross-cutting plumbing for editorial packs
([discover-tab-retroverse.md](discover-tab-retroverse.md)) and any
future pack-shaped content (themes, per-system UI asset bundles,
cheats, metadata enrichment). Cores are explicitly **excluded** —
libretro's existing buildbot + RetroArch update flow handles that
ecosystem.

---

## 1. TL;DR

A single mechanism for distributing optional content packs to OA
installs. Operator-initiated only — no background fetches, no
silent updates.

Locked v1 model:

- **One OA-curated registry** hosted as a plain JSON file in a
  public GitHub repo (no server infra).
- **sha256 hash verification** against the registry-listed hash.
  Mismatch = reject the download.
- **No built-in packs** in the installer. First-launch OA has
  zero pack content; empty states offer to install the
  appropriate baseline pack.
- **Auto-update never.** Operator clicks "Check for updates"
  manually.
- **Privacy panel** discloses every URL OA will ever hit and
  when, with a master "Allow network calls" toggle defaulting
  to ON but easy to flip OFF.
- **First pack to land:** OA Editorial Baseline (DISCOVER
  content — articles, spotlights, anniversaries, axis-framing
  essays). CC-BY-SA-4.0 so the community can fork + extend.

Self-hosted packs, federation, and cryptographic signing are
**deferred** — revisit once a few trusted community packs exist.

---

## 2. Scope

What this mechanism covers:

| Pack type | Status | First version |
|---|---|---|
| **Editorial** (DISCOVER content) | ships first | OA Editorial Baseline |
| **Themes** (when shells become swappable) | future | TBD |
| **Per-system UI asset bundles** (boot anims, SFX, backgrounds) | future | extends per-system-ui Stage 1's ASSETS catalog |
| **Cheats** (Game Genie / PAR code databases) | future | if cheats ever land |
| **Metadata enrichment** (release dates, descriptions) | future | LaunchBox-style sync supplement |

What this mechanism does **not** cover:

- **Cores** — libretro `.dll`s. RetroArch already has a
  perfectly good buildbot + in-app updater for this; OA reuses
  that ecosystem. Operators drop `.dll`s into `<exe_dir>/cores/`
  manually (or via RetroArch's update flow if they have it
  installed). SETTINGS → Cores in OA is a *status view* only —
  installed versions, last-modified — not an updater.
- **BIOS files** — operator legally acquires their own per the
  licensing constraints.
- **ROMs** — same.

---

## 3. Constraints (from CLAUDE.md + project ethos)

- **"No network calls from emulator code."** Strict for the
  emulator runtime. The shell sits outside that rule, but the
  *spirit* — fully offline, operator owns their data — holds.
  Every network call the pack mechanism makes is operator-
  initiated and disclosed.
- **Portable install supported.** Packs must work both in
  AppData mode and in `<exe_dir>/settings/` mode.
- **Non-commercial.** No DRM, no paywalls, no telemetry, no
  analytics on pack installs. Pack downloads are anonymous —
  OA does not phone home with operator IDs.
- **Operator-initiated, never silent.** No background polling,
  no first-launch registry fetch. The first network call ever
  is the moment the operator clicks "Browse available packs"
  (or "Check for updates").

---

## 4. Distribution model — OA-curated single registry

The registry is a single JSON file living in a public GitHub
repo controlled by the OA team:

```
https://raw.githubusercontent.com/overlooked-arcade/oa-pack-registry/main/registry.json
```

No server to run; no infra to maintain. GitHub serves the file
free, with reasonable rate limits and uptime. Community
contributes via PRs against that repo — the OA team reviews and
merges. Same trust model as the OA source code itself.

### Registry shape

```json
{
  "registry_version": 1,
  "updated": "2026-05-28T12:00:00Z",
  "packs": [
    {
      "id": "oa-editorial-baseline",
      "type": "editorial",
      "name": "OA Editorial Baseline",
      "summary": "20+ articles, system spotlights, anniversaries.",
      "version": "0.3.0",
      "maintainer": "overlooked-arcade",
      "license": "CC-BY-SA-4.0",
      "url": "https://github.com/overlooked-arcade/oa-editorial-baseline/releases/download/v0.3.0/pack.zip",
      "sha256": "abc123def456...",
      "size_bytes": 14238921,
      "depends_on": [],
      "min_oa_version": "0.9.0",
      "homepage": "https://github.com/overlooked-arcade/oa-editorial-baseline"
    }
  ]
}
```

Fields:

- `id` — globally unique pack id, slug form.
- `type` — `editorial` / `theme` / `system-ui-assets` / `cheats`
  / `metadata`. OA dispatches install location by type.
- `version` — semver. Used for update detection.
- `url` — direct download URL for the pack zip. Pinned to a
  specific release; never moves.
- `sha256` — expected hash of the downloaded zip. **The
  trust anchor for pack integrity.**
- `size_bytes` — for download progress + sanity check.
- `depends_on` — other pack ids required for this pack to load.
  Install ordering follows the DAG.
- `min_oa_version` — refuse to install if OA is older.
- `license` — must be explicit. Operator-visible so they know
  what they're consuming.

### Why a single registry, not federated

Federation (operator adds N custom registry URLs) gets deferred
because:

- Trust model is on the operator with no UI to help them judge
  it.
- We have no community packs yet — federation solves a problem
  that doesn't exist.
- Single registry = single PR review queue = OA team can
  enforce minimum-quality + licensing standards.

When trusted community packs exist + the demand for self-
hosting is real, federation lands as a follow-up. The registry
schema is already set up to make this drop-in (each pack just
gains an implicit "source" field).

---

## 5. Verification — sha256 hash check

Every pack listed in the registry carries an `sha256` field
holding the expected hash of the downloaded zip.

Install flow:

1. OA fetches the zip from the registry-listed `url`.
2. OA computes sha256 of the downloaded bytes.
3. Compare against the registry-listed `sha256`.
4. **Mismatch → reject the download, surface error to operator,
   no install.**
5. Match → extract to staging dir, validate manifest, move into
   the community pack folder.

This catches:

- Transit corruption (rare but possible).
- A registry-listed pack being swapped on GitHub after the
  registry was last updated.
- Most accidental misconfigurations.

It does **not** catch:

- A compromised OA registry serving malicious hashes for
  malicious zips.
- A compromised pack maintainer pushing a malicious update
  through the normal PR flow.

These are addressed later by cryptographic signing (deferred).
For v1, the trust model is: *"OA team reviews registry PRs;
operator trusts the OA team."* — same model as trusting the OA
installer itself.

---

## 6. Pack zip structure

Every pack is a zip with a top-level `manifest.yml` and
type-specific content directories.

```
oa-editorial-baseline-0.3.0.zip
├── manifest.yml
├── articles/
│   ├── tg16-almost-was.md
│   ├── konami-years.md
│   └── ...
├── spotlights/
│   ├── tg16.md
│   ├── gameboy.md
│   └── ...
├── axes/
│   ├── eras.yml
│   ├── genres.yml
│   ├── regions.yml
│   └── developers.yml
├── anniversaries.yml   (optional — supplements release_date data)
└── assets/
    ├── tg16-almost-was/hero.jpg
    └── ...
```

### `manifest.yml`

```yaml
id: oa-editorial-baseline
version: 0.3.0
type: editorial
maintainer: overlooked-arcade
license: CC-BY-SA-4.0
license_url: https://creativecommons.org/licenses/by-sa/4.0/
summary: "20+ curated articles, system spotlights, anniversaries."
homepage: https://github.com/overlooked-arcade/oa-editorial-baseline
min_oa_version: "0.9.0"
contents:
  articles: 23
  spotlights: 12
  axes: [eras, genres, regions, developers]
```

OA validates the manifest against the registry entry on install
(name, id, version, type must match). Refuse mismatches.

---

## 7. Layered loading

Content type → directory mapping:

```
<exe_dir>/discover/
  community/<pack_id>/        # installed via pack manager
  overrides/                  # operator-edited drop-ins

<exe_dir>/themes/
  community/<pack_id>/
  overrides/

<exe_dir>/assets/system-ui/<system_id>/
  community/<pack_id>/
  overrides/
```

Loading order: `community/<pack_id>/` (alphabetical, or by install
order) → `overrides/`. Last wins.

**No `builtin/` tier** — OA ships with zero pack content. Empty
states in DISCOVER / theme picker / etc. offer "Browse available
packs" instead of pretending there's built-in content.

The `overrides/` tier lets the operator edit any article (or
override any asset) without forking the source pack. Drop a
matching path in there; it wins. Useful for personal annotations
or fixing typos without losing them on the next pack update.

### Conflicts between community packs

If two installed packs declare the same article ID (etc.), the
*last-installed* wins by default, but OA surfaces a non-fatal
warning in the SETTINGS → Content panel: `⚠ ARTICLE_ID conflict
between PACK_A and PACK_B — PACK_B wins`. Operator can resolve
manually by uninstalling one or dropping a tie-breaker in
`overrides/`.

---

## 8. Install / update / uninstall flows

### Install

1. Operator opens SETTINGS → Content → Packs.
2. Clicks **"Browse available packs"**.
3. OA fetches the registry JSON (first network call). Renders
   the list.
4. Operator picks a pack, clicks **Install**.
5. OA downloads the zip → verifies sha256 → extracts to a
   *staging* dir under `<data_dir>/staging/<pack_id>-<uuid>/`
   → validates manifest → moves atomically into
   `<exe_dir>/<type>/community/<pack_id>/`.
6. Content index reloads. Pack content is live.

### Update

1. Same panel, **"Check for updates"** button.
2. OA fetches the registry JSON.
3. For each installed pack, compares installed version to
   registry version.
4. Updates surface as `▲ Update available — v0.3.0 → v0.4.0`
   rows.
5. Operator picks which to apply; same download → verify →
   stage → atomic-swap flow as install.
6. Old version is retained for one cycle under
   `<data_dir>/packs-rollback/<pack_id>-<version>/` so the
   operator can roll back from the same panel for 14 days.
7. Older rollbacks are GC'd at the same panel.

### Uninstall

1. Same panel, **Uninstall** button per pack.
2. OA confirms — "this removes content this pack provided. Your
  saved-for-later list and overrides are preserved."
3. Remove `<exe_dir>/<type>/community/<pack_id>/`. Move the
  most-recent version into rollback retention. Reload content
  index.

---

## 9. SETTINGS surfaces

Two new categories in the SETTINGS tab (per
[settings-tab-retroverse.md](settings-tab-retroverse.md) — these
fold into the CONTENT group):

### SETTINGS → Content → Packs

Three sections:

- **Installed** — list of installed packs with version, last-
  updated date, license, **Uninstall** action, **Rollback** if
  a prior version is in retention.
- **Available** — registry contents, filtered to packs the
  operator does not yet have installed. **Install** action per
  row.
- **Updates** — packs with newer registry versions. **Update
  all** + **Update this** actions.

Top of the panel: **Check for updates** + **Last checked: <date>**
+ a **Registry URL** display.

### SETTINGS → Privacy

A new category in the SYSTEM group. Discloses every URL OA will
ever hit and when:

```
OA never contacts any server unless you ask it to.

When you click these buttons in Content → Packs, OA hits:
  - Browse / Check for updates → https://raw.githubusercontent.com/
                                 overlooked-arcade/oa-pack-registry/
                                 main/registry.json
  - Install / Update            → the registry-listed URL for the
                                 specific pack (always a GitHub
                                 Releases URL).

OA does not send your operator ID, machine fingerprint, IP-derived
location, telemetry, or any other identifying data. The downloads
are anonymous HTTP GETs.

[ ●ON  OFF ]  Allow network calls
              (when OFF: pack browse / install / update are disabled)

[ Show network log ]  ← per-call audit trail of every URL OA has
                         hit and when. Empty on first launch.
```

Master toggle defaults to ON but is one click away from OFF.
When OFF, pack-related buttons in Content → Packs go disabled
with a tooltip pointing back here.

---

## 10. First pack — OA Editorial Baseline

Ships out-of-band from OA itself, via the registry, after the
DISCOVER UI lands. Content:

- ~20 articles covering system histories, lost games, cult
  classics, anniversaries.
- ~12 system spotlights (one per first-wave system at minimum).
- Axis-framing essays for eras, genres, regions, developers.
- License: **CC-BY-SA-4.0**. Community can fork freely; OA team
  curates the canonical baseline.
- Maintainer: `overlooked-arcade` GitHub org.

The pack is **fully optional**. DISCOVER without it shows an
empty state per axis:

```
┌─ ▣ FEATURED ─────────────────────┐
│                                  │
│   No editorial content installed.│
│                                  │
│   The OA Editorial Baseline pack │
│   adds ~20 articles, system      │
│   spotlights, and anniversary    │
│   coverage.                      │
│                                  │
│   [ ▶ Browse available packs ]   │
│                                  │
└──────────────────────────────────┘
```

Same shape for the other axes. No broken-feeling empty states —
each axis explains what content would fill it.

---

## 11. Open questions (revisit when relevant)

- **Federation.** Add a "Custom registries" surface in
  SETTINGS → Content once trusted community packs exist + at
  least one operator asks for self-hosting.
- **Cryptographic signing.** Add minisign / sigstore once
  community packs are common enough that hash-from-registry
  isn't the only trust anchor.
- **Translation / locale packs.** Editorial in English-only at
  first; structure for per-locale articles
  (`articles/<lang>/<id>.md`) reserved.
- **Pack categories / search.** Registry currently a flat list.
  Once it grows past ~10 packs, add tags + a registry-side
  category schema.
- **Operator-published packs.** Workflow for an operator to
  publish their own pack to the OA registry via PR — needs a
  pack-template repo + a contributing.md.
- **Mirroring.** If GitHub goes down, what happens? Defer until
  it actually matters; consider a CDN mirror at that point.

---

## 12. Implementation sketch (not committed)

Not a green-lit implementation plan — rough mapping:

- Rust side: new `oa-packs` crate. Public API: `list_installed()`,
  `fetch_registry()`, `install(pack_id)`, `update(pack_id)`,
  `uninstall(pack_id)`, `verify(zip_path, expected_sha)`.
- HTTP client: `reqwest` with `default-features = false` +
  `rustls-tls` (no system OpenSSL dependency). Single client
  instance used only when operator-initiated calls fire.
- Tauri commands: `oa_packs_list`, `oa_packs_fetch_registry`,
  `oa_packs_install`, `oa_packs_update`, `oa_packs_uninstall`,
  `oa_packs_get_privacy_settings`, `oa_packs_set_privacy_settings`.
- Frontend: SETTINGS → Content → Packs panel + SETTINGS →
  Privacy panel. New `lib/packs.ts` service wrapping the Tauri
  commands.
- Network log: ring buffer in `oa-packs` state, persisted to
  `<data_dir>/packs/network.log` (last 100 entries). Surfaced
  in SETTINGS → Privacy → Show network log.
- Allow-network toggle stored in OA prefs. When OFF, all
  Tauri commands that hit the network return
  `Err(NetworkDisabled)` synchronously without making the call.

Status: idea, not in `ACTIVE_WORK.md`. Implementation order
(when greenlit):

1. `oa-packs` crate scaffolding + manifest validation + sha256
   verification (no network yet — install from local zip path).
2. Registry fetch + Settings panel listing installed/available
   packs.
3. End-to-end install / update / uninstall flows.
4. Privacy panel + allow-network toggle + network log.
5. OA Editorial Baseline pack content + first registry entry.
6. DISCOVER UI consumes installed pack content.

Steps 1-4 ship as a feature with no visible-to-end-user payoff
until step 5-6 light it up — but they unblock every future
pack-shaped content stream.
