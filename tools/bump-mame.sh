#!/usr/bin/env bash
# bump-mame.sh — refresh the slim MAME data files in assets/mame-source/
# from a locally-installed MAME.
#
# Maintainer workflow. Run this when a new MAME release ships better
# data for the consoles OA covers (occasionally the case — recent MAME
# trees improved CPU clocks for several Sega + Atari machines). The
# files produced here ship in the OA installer; operators don't run
# bump-mame.sh themselves (the in-app "Refresh MAME system info"
# button in Phase 5 handles operator-driven re-imports against their
# own MAME install).
#
# Usage:
#   tools/bump-mame.sh                          (auto-detects MAME)
#   MAME=/path/to/mame.exe tools/bump-mame.sh   (override MAME binary)
#   HISTORY=/path/to/history.xml tools/bump-mame.sh
#                                               (override history.xml)
#
# Outputs:
#   assets/mame-source/listxml-slim.json
#   assets/mame-source/history-slim.xml
#   assets/mame-source/mame-version.txt
#
# Reference: docs/PLANS/system-info-panel-v1.md §4.

set -euo pipefail

# Resolve script directory (works under MSYS / Git Bash / WSL).
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
ASSETS_DIR="$REPO_ROOT/assets/mame-source"
EXTRACTOR_MANIFEST="$SCRIPT_DIR/mame-extractor/Cargo.toml"

# --- Locate MAME ------------------------------------------------------
# Canonical location: <repo_root>/Emulators/MAME/ — the project-wide
# convention for all third-party emulator binaries OA shells out to.
# Maintainers drop MAME here; the shipped install probes the same
# relative path under <exe_dir>/Emulators/MAME/ in Phase 5's in-app
# re-import flow. Falls through to PATH + common system locations only
# when the repo-local copy is absent.
# Override via $MAME if the maintainer's install lives somewhere else.
MAME="${MAME:-}"
if [[ -z "$MAME" ]]; then
    for candidate in \
        "$REPO_ROOT/Emulators/MAME/mame.exe" \
        "$REPO_ROOT/Emulators/MAME/mame" \
        "mame.exe" \
        "mame" \
        "/c/mame/mame.exe" \
        "/c/Program Files/MAME/mame.exe" \
        "/usr/bin/mame" \
        "/usr/local/bin/mame"
    do
        if [[ -x "$candidate" ]] || command -v "$candidate" >/dev/null 2>&1; then
            MAME="$candidate"
            break
        fi
    done
fi

if [[ -z "$MAME" ]] || { [[ ! -x "$MAME" ]] && ! command -v "$MAME" >/dev/null 2>&1; }; then
    cat >&2 <<EOF
bump-mame.sh: MAME not found.
Canonical location for this project is $REPO_ROOT/Emulators/MAME/mame.exe
(or 'mame' on Linux). Drop your MAME install there and re-run.
Also tried: \$MAME env var, PATH lookups for mame.exe + mame, plus
       /c/mame/mame.exe, /c/Program Files/MAME/mame.exe,
       /usr/bin/mame, /usr/local/bin/mame.
Pass the binary explicitly: MAME=/full/path/to/mame.exe $0
EOF
    exit 1
fi

echo "bump-mame.sh: using MAME at: $MAME"

# --- Capture MAME version --------------------------------------------
# `mame -version` prints something like "MAME v0.262 (Jul 31 2024)".
# We want a stable short string; strip the date stamp.
RAW_VERSION="$("$MAME" -version 2>&1 | head -n1)"
SHORT_VERSION="$(echo "$RAW_VERSION" | sed -E 's/^MAME v?([0-9.]+).*/\1/' | tr -d '[:space:]')"
if [[ -z "$SHORT_VERSION" ]]; then
    # Fall back to the raw output — better to ship something than nothing.
    SHORT_VERSION="$RAW_VERSION"
fi
echo "bump-mame.sh: MAME version: $SHORT_VERSION"

# --- Locate history.xml ----------------------------------------------
# MAME normally bundles history.xml under <mame_dir>/history/history.xml,
# but some packagings drop it at the root or omit it entirely.
HISTORY="${HISTORY:-}"
if [[ -z "$HISTORY" ]]; then
    MAME_DIR="$(dirname "$(command -v "$MAME" 2>/dev/null || echo "$MAME")")"
    # Repo-local canonical location first, then derive from wherever
    # MAME was found. The repo-local check stays useful even when $MAME
    # was overridden to a system install — operators sometimes pair a
    # system-wide MAME with a project-tracked history.xml.
    for candidate in \
        "$REPO_ROOT/Emulators/MAME/history/history.xml" \
        "$REPO_ROOT/Emulators/MAME/history.xml" \
        "$MAME_DIR/history/history.xml" \
        "$MAME_DIR/history.xml" \
        "$MAME_DIR/dats/history.xml"
    do
        if [[ -f "$candidate" ]]; then
            HISTORY="$candidate"
            break
        fi
    done
fi

if [[ -z "$HISTORY" ]] || [[ ! -f "$HISTORY" ]]; then
    # history.xml is community-maintained (progettoSNAPS / Pleasuredome
    # ship it separately from MAME proper) — many maintainers don't have
    # it on hand. Continue without it; the extractor emits a placeholder
    # history-slim.xml and the Phase 2 loader treats every L1 record as
    # description-less. Operator can drop history.xml at the canonical
    # path below and re-run bump-mame.sh later to populate it.
    cat >&2 <<EOF
bump-mame.sh: history.xml not found — continuing WITHOUT description data.
Canonical location for this project: $REPO_ROOT/Emulators/MAME/history/history.xml
Source: arcade-history.com -> "History.xml for MAME" download page.
MAME deprecated the legacy history.dat text format in 2023; only the
XML form is published today (history.xml v2.87a+).
Drop a copy at the canonical path above (or set \$HISTORY) and re-run
this script to bake richer per-system descriptions into L1.
EOF
    HISTORY=""
else
    echo "bump-mame.sh: history.xml at: $HISTORY"
fi

# --- Run MAME -listxml -----------------------------------------------
# The output is 200MB+ on recent releases. Use a temp file so the
# extractor can stream-parse via Reader::from_file rather than holding
# the entire string in RAM via stdin.
LISTXML_TMP="$(mktemp -t mame-listxml.XXXXXX.xml)"
trap 'rm -f "$LISTXML_TMP"' EXIT

echo "bump-mame.sh: running mame -listxml (this takes ~15s)…"
"$MAME" -listxml > "$LISTXML_TMP"

# --- Build + run the extractor ---------------------------------------
mkdir -p "$ASSETS_DIR"

echo "bump-mame.sh: building extractor (release)…"
cargo build --release --manifest-path "$EXTRACTOR_MANIFEST" >&2

EXTRACTOR_BIN="$SCRIPT_DIR/mame-extractor/target/release/mame-extractor"
if [[ ! -x "$EXTRACTOR_BIN" ]] && [[ -f "${EXTRACTOR_BIN}.exe" ]]; then
    EXTRACTOR_BIN="${EXTRACTOR_BIN}.exe"
fi

echo "bump-mame.sh: extracting slim data…"
EXTRACTOR_ARGS=(
    --listxml "$LISTXML_TMP"
    --mame-version "$SHORT_VERSION"
    --out "$ASSETS_DIR"
)
if [[ -n "$HISTORY" ]]; then
    EXTRACTOR_ARGS+=(--history "$HISTORY")
fi
"$EXTRACTOR_BIN" "${EXTRACTOR_ARGS[@]}"

echo "bump-mame.sh: done. Artifacts in $ASSETS_DIR/"
ls -l "$ASSETS_DIR"
