// UnidentifiedGamesDialog — operator audit surface for games in a system
// whose sha1 is NULL / empty (i.e. Identify ROMs hasn't matched them).
//
// Surfaces from LibraryManagerPage's GameMediaManagePanel via the
// "View N unidentified ▸" affordance on the Identify ROMs op row.
// Shows enough per-row context for the operator to diagnose why a row
// didn't match (filename, archive inner path, has-disc-id flag) and a
// "Show in folder" button to inspect the file on disk.
//
// Doesn't include a manual-identify input form — v1 is read-only +
// reveal-in-OS. If the operator wants to act, they fix the underlying
// issue (rename file, add missing .bin sidecar, etc.) and re-run
// "Identify ROMs" via the footer button.

import { createMemo, createResource, For, Show, type Component } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { Dialog } from "@oa/platform/components/Dialog";
import { reportInvokeError } from "@oa/platform/lib/toast";
import { systemThemes, type SystemId } from "@oa/platform/themes/registry";

export type UnidentifiedGameRow = {
  id: string;
  systemId: string;
  title: string;
  filePath: string;
  archiveInnerPath?: string | null;
  hasDiscId: boolean;
};

export type UnidentifiedGamesDialogProps = {
  /// Active system id. When null, the dialog is closed.
  systemId: SystemId | null;
  /// Total game count for the system — drives the "N of M total" header.
  totalGames: number;
  /// Triggers the existing Identify ROMs flow on the parent; the parent
  /// closes the dialog and refreshes counts when the resolve completes.
  onRunIdentify: (id: SystemId) => void;
  onClose: () => void;
};

/// Hard cap on rendered rows. Most operators won't hit this; if they do,
/// the count surfaces and they can use the system-wide Identify ROMs to
/// chip away at the list.
const RENDER_CAP = 500;

const UnidentifiedGamesDialog: Component<UnidentifiedGamesDialogProps> = (props) => {
  const systemName = createMemo(() => {
    const id = props.systemId;
    if (!id) return "";
    return systemThemes[id]?.displayName ?? id;
  });

  // Re-fetch when systemId changes. createResource handles caching +
  // refetch automatically when the source signal updates.
  const [rows] = createResource(
    () => props.systemId,
    async (id): Promise<UnidentifiedGameRow[]> => {
      if (!id) return [];
      try {
        return await invoke<UnidentifiedGameRow[]>("list_unidentified_games", {
          systemId: id,
        });
      } catch (e) {
        reportInvokeError("list_unidentified_games", e);
        return [];
      }
    },
  );

  const visible = createMemo(() => {
    const list = rows() ?? [];
    return list.slice(0, RENDER_CAP);
  });

  const headerSubtitle = createMemo(() => {
    const total = rows()?.length ?? 0;
    if (rows.loading) return "Loading…";
    if (total === 0) return `Everything in ${systemName()} is identified. 🎉`;
    const shown = visible().length;
    const totalGames = props.totalGames;
    const totalSuffix = totalGames > 0 ? ` of ${totalGames} total` : "";
    if (shown < total) {
      return `Showing first ${shown} of ${total} unidentified${totalSuffix}`;
    }
    return `${total} unidentified${totalSuffix}`;
  });

  function onShowInFolder(filePath: string) {
    void invoke("reveal_game_file_in_folder", { filePath }).catch((e) =>
      reportInvokeError("reveal_game_file_in_folder", e),
    );
  }

  function onRunIdentify() {
    const id = props.systemId;
    if (!id) return;
    props.onRunIdentify(id);
    props.onClose();
  }

  return (
    <Dialog
      open={props.systemId !== null}
      onClose={props.onClose}
      title={`Unidentified games — ${systemName()}`}
      subtitle={headerSubtitle()}
      system={props.systemId ?? undefined}
      size="xl"
    >
      <div class="flex flex-col gap-4">
        <Show
          when={!rows.loading && (rows()?.length ?? 0) > 0}
          fallback={
            <Show
              when={!rows.loading}
              fallback={<p class="text-sm text-(--color-oa-ink-dim)">Loading…</p>}
            >
              <p class="text-sm text-(--color-oa-ink-dim)">
                No unidentified games. Every ROM in this system has a sha1
                stamped from a canonical catalog match.
              </p>
            </Show>
          }
        >
          <p class="text-xs leading-relaxed text-(--color-oa-ink-dim)">
            These ROMs didn't match a canonical entry on the last Identify
            ROMs pass. Common causes: filename diverges from the no-intro /
            redump catalog name (rename to match), a .cue is missing its
            .bin sidecar (use Show in folder to check), the system's hash
            DB is empty (run Refresh hash database first), or the title
            genuinely isn't in the catalog. Re-run Identify ROMs after
            fixing the underlying issue.
          </p>

          <ul class="flex max-h-[55vh] flex-col gap-2 overflow-y-auto rounded-lg border border-white/10 bg-white/[0.02] p-2">
            <For each={visible()}>
              {(row) => (
                <li class="flex items-start justify-between gap-3 rounded-md border border-white/5 bg-white/[0.03] px-3 py-2">
                  <div class="min-w-0 flex-1">
                    <p class="truncate text-[0.8rem] font-semibold text-(--color-oa-ink)">
                      {row.title}
                    </p>
                    <p
                      class="mt-0.5 truncate text-[0.65rem] text-(--color-oa-ink-dim)"
                      title={row.filePath}
                    >
                      {row.filePath}
                    </p>
                    <Show when={row.archiveInnerPath}>
                      <p
                        class="truncate text-[0.65rem] text-(--color-oa-ink-dim)/80"
                        title={row.archiveInnerPath ?? undefined}
                      >
                        ↳ {row.archiveInnerPath}
                      </p>
                    </Show>
                    <Show when={row.hasDiscId}>
                      <p class="mt-1 inline-flex items-center rounded-md border border-amber-400/30 bg-amber-400/10 px-1.5 py-0.5 text-[0.55rem] uppercase tracking-widest text-amber-200">
                        Disc-ID stamped — needs fuzzy re-pass
                      </p>
                    </Show>
                  </div>
                  <button
                    type="button"
                    onClick={(e) => {
                      e.currentTarget.blur();
                      onShowInFolder(row.filePath);
                    }}
                    class="shrink-0 rounded-md border border-white/10 bg-white/[0.04] px-2.5 py-1 text-[0.6rem] font-semibold uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:border-white/20 hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
                  >
                    Show in folder
                  </button>
                </li>
              )}
            </For>
          </ul>
        </Show>

        <footer class="flex items-center justify-end gap-2 border-t border-white/5 pt-3">
          <button
            type="button"
            onClick={(e) => {
              e.currentTarget.blur();
              props.onClose();
            }}
            class="rounded-md border border-white/10 bg-white/[0.04] px-4 py-1.5 text-[0.7rem] font-semibold uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:border-white/20 hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
          >
            Done
          </button>
          <Show when={(rows()?.length ?? 0) > 0}>
            <button
              type="button"
              onClick={(e) => {
                e.currentTarget.blur();
                onRunIdentify();
              }}
              class="rounded-md border border-(--color-system-accent)/40 bg-(--color-system-accent)/15 px-4 py-1.5 text-[0.7rem] font-semibold uppercase tracking-wider text-(--color-oa-ink) transition hover:border-(--color-system-accent) hover:bg-(--color-system-accent)/25"
            >
              Re-run Identify ROMs
            </button>
          </Show>
        </footer>
      </div>
    </Dialog>
  );
};

export default UnidentifiedGamesDialog;
