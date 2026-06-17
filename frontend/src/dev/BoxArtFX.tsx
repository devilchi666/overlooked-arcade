// BoxArtFX — DEV-ONLY box-art treatment bench (Theming ARC 3 Thrust M, M0).
// The "true yes-we-can" tab: real cover art from the user's library with premium
// finishes — reflection + grounded shadow, and a glass/gloss finish — plus
// pointer-tilt so they feel tactile. Pulls actual covers via useMedia(); cycle
// through your library with "Next art".
//
// Never ships (gated behind import.meta.env.DEV at the App mount).

import { createMemo, createSignal, Show, type JSX } from "solid-js";
import { usePlatform } from "@oa/platform/platformContext";
import { useMedia } from "@oa/platform/library/media";
import type { RomEntry } from "@oa/platform/library/types";

const KEYFRAMES = `
@keyframes oa-glass-sweep { from{background-position:-160% 0} to{background-position:260% 0} }
`;

/// Pointer-tilt: a 3D rotate that follows the cursor, eased back on leave.
function useTilt(maxDeg = 16) {
  const [t, setT] = createSignal("perspective(900px)");
  const onMove = (e: MouseEvent & { currentTarget: HTMLElement }): void => {
    const r = e.currentTarget.getBoundingClientRect();
    const px = (e.clientX - r.left) / r.width - 0.5;
    const py = (e.clientY - r.top) / r.height - 0.5;
    setT(`perspective(900px) rotateY(${px * maxDeg}deg) rotateX(${-py * maxDeg}deg)`);
  };
  const onLeave = (): void => {
    setT("perspective(900px)");
  };
  return { t, onMove, onLeave };
}

export default function BoxArtFX(): JSX.Element {
  const platform = usePlatform();
  const media = useMedia();

  const coverFor = (e: RomEntry): string | null =>
    (e.identityId ? media.coverUrl(e.systemId, e.identityId) : null) ??
    media.coverUrl(e.systemId, e.id);

  // All library entries that actually have a cover image.
  const withCovers = createMemo(() =>
    platform.library.state.entries
      .map((e) => ({ e, url: coverFor(e) }))
      .filter((x): x is { e: RomEntry; url: string } => !!x.url),
  );

  const [idx, setIdx] = createSignal(0);
  const current = createMemo(() => {
    const list = withCovers();
    if (list.length === 0) return null;
    return list[idx() % list.length];
  });
  const url = (): string | null => current()?.url ?? null;
  const title = (): string => current()?.e.title ?? "No cover in library";

  const next = (): void => {
    setIdx((i) => i + 1);
  };

  const tiltA = useTilt();
  const tiltB = useTilt(18);

  // The art face — real <img> when a cover exists, else a gradient placeholder
  // so the FX still demos on an empty library.
  const Face = (p: { flip?: boolean; style?: JSX.CSSProperties }): JSX.Element => (
    <Show
      when={url()}
      fallback={
        <div
          class="h-64 w-48 rounded-lg"
          style={{ "background-image": "linear-gradient(135deg,#7c3aed,#22d3ee)", transform: p.flip ? "scaleY(-1)" : undefined, ...p.style }}
        />
      }
    >
      {(u) => (
        <img
          src={u()}
          alt=""
          class="block h-64 w-48 rounded-lg object-cover"
          style={{ transform: p.flip ? "scaleY(-1)" : undefined, ...p.style }}
        />
      )}
    </Show>
  );

  return (
    <div class="flex h-full min-h-0 flex-col overflow-y-auto pr-1">
      <style>{KEYFRAMES}</style>

      <div class="mb-4 flex flex-wrap items-center gap-3">
        <h3 class="text-sm font-bold">Box art FX</h3>
        <button class="rounded border border-white/20 px-2.5 py-1 text-xs hover:bg-white/10" onClick={next}>Next art →</button>
        <span class="text-[0.62rem] text-white/55">
          {withCovers().length > 0
            ? <>“{title()}” · {withCovers().length} covers in library · hover to tilt</>
            : <>No covers found in your library — showing a placeholder. Sync covers to see your art.</>}
        </span>
      </div>

      <div class="grid flex-1 grid-cols-1 gap-6 lg:grid-cols-2">
        {/* Reflection + grounded shadow */}
        <div class="flex flex-col rounded-xl border border-white/10 bg-gradient-to-b from-[#11151c] to-[#05070b] p-6">
          <p class="mb-4 text-xs font-semibold text-white/70">Reflection + shadow</p>
          <div class="flex flex-1 items-center justify-center">
            <div
              class="relative"
              style={{ transform: tiltA.t(), transition: "transform 140ms ease-out" }}
              onMouseMove={tiltA.onMove}
              onMouseLeave={tiltA.onLeave}
            >
              {/* grounded contact shadow */}
              <div
                class="absolute -bottom-3 left-1/2 h-6 w-44 -translate-x-1/2 rounded-[50%]"
                style={{ background: "radial-gradient(ellipse at center, rgba(0,0,0,.7), transparent 70%)", filter: "blur(6px)" }}
              />
              {/* the cover, with a cast drop-shadow */}
              <Face style={{ filter: "drop-shadow(0 22px 30px rgba(0,0,0,.7))" }} />
              {/* reflection — flipped copy, faded with a mask */}
              <div class="absolute left-0 top-full w-48 overflow-hidden" style={{ "margin-top": "5px" }}>
                <Face
                  flip
                  style={{
                    opacity: "0.45",
                    "transform-origin": "top",
                    "mask-image": "linear-gradient(to bottom, rgba(0,0,0,.6), transparent 58%)",
                    "-webkit-mask-image": "linear-gradient(to bottom, rgba(0,0,0,.6), transparent 58%)",
                  }}
                />
              </div>
            </div>
          </div>
        </div>

        {/* Glass / gloss finish */}
        <div class="flex flex-col rounded-xl border border-white/10 bg-gradient-to-b from-[#0c1019] to-[#05070b] p-6">
          <p class="mb-4 text-xs font-semibold text-white/70">Glass finish</p>
          <div class="flex flex-1 items-center justify-center">
            <div
              class="relative h-64 w-48 overflow-hidden rounded-xl"
              style={{ transform: tiltB.t(), transition: "transform 140ms ease-out", "box-shadow": "0 20px 40px rgba(0,0,0,.6)" }}
              onMouseMove={tiltB.onMove}
              onMouseLeave={tiltB.onLeave}
            >
              <Face style={{ height: "100%", width: "100%" }} />
              {/* frost + edge gloss (backdrop-filter blurs the art behind it) */}
              <div
                class="pointer-events-none absolute inset-0 rounded-xl"
                style={{
                  "backdrop-filter": "blur(1.4px) saturate(1.15)",
                  "background-image": "linear-gradient(135deg, rgba(255,255,255,.38), rgba(255,255,255,0) 38%, rgba(255,255,255,.05) 60%, rgba(255,255,255,.24))",
                  "box-shadow": "inset 0 1px 0 rgba(255,255,255,.55), inset 0 0 0 1px rgba(255,255,255,.18)",
                }}
              />
              {/* moving specular highlight */}
              <div
                class="pointer-events-none absolute inset-0 rounded-xl"
                style={{
                  "background-image": "linear-gradient(110deg, transparent 36%, rgba(255,255,255,.5) 50%, transparent 64%)",
                  "background-size": "250% 100%",
                  animation: "oa-glass-sweep 3.6s linear infinite",
                }}
              />
            </div>
          </div>
        </div>
      </div>

      <p class="mt-4 text-[0.6rem] text-white/35">
        Real covers, premium finishes, on the live surface. If these read right,
        that's the "yes we can do it." Reflection/glass become reusable tile
        treatments themes opt into; tilt is an optional per-tile flourish.
      </p>
    </div>
  );
}
