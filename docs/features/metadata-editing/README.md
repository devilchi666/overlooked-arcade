# Metadata Curation

A premium operator surface in engine **Settings → "Metadata"** to edit **game**
and **system** metadata, stored as an **override layer** over synced/shipped
facts (per-field reset + provenance). OA's biggest greenfield interaction win
over LaunchBox (research §4 / §10 Q2).

- **Plan (authoritative):** [../../PLANS/metadata-editing.md](../../PLANS/metadata-editing.md)
- **Decisions:** [DECISIONS.md](DECISIONS.md)
- **Session log:** [SESSION_LOG.md](SESSION_LOG.md)

**Status (2026-06-11):** planned, not started. Wave 1 / S1 (game-factual
override backend) queued in `docs/NEXT.md` HIGH band.

**Shape:** Wave 1 = the editor (override backend for game-factual metadata +
the Settings category + system & game editors, premium UX). Wave 2 = undo +
merge-mode bulk edit / find-and-replace. Wave 3 (deferred) = fix-wrong-match,
media picker, inline-in-library editing.

**Key facts:** system-metadata override backend is **already shipped**
(`system_info_overrides`) — system editing is a UI job. Game-factual override
backend must be **built** to mirror the shipped `game_info_overrides` pattern.
Edits key on the **identity**. Editor lives in **engine/Settings** (theme-free).
