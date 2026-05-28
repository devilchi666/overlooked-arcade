// Retroverse-UI Phase B Slice 5 — "Coming soon" placeholder page.
//
// Used by the five non-LIBRARY tabs in Phase B (HOME / COLLECTIONS /
// PLAY NOW / DISCOVER / SETTINGS) so the routing is observably correct
// even though only LIBRARY has a real implementation.
//
// Each stub names its tab + the design doc that covers it so the
// operator can confirm "yes, this tab will exist" without confusion.
// Phase C replaces each stub with the real page; this file goes away
// once all five have shipped.

import type { Component } from "solid-js";
import { HintRegion } from "../../nav/HintBar";

type Props = {
  /// Tab name as it appears in the top-tab strip (capitalized).
  title: string;
  /// Filename of the design doc covering this tab. Rendered as a
  /// monospace hint so the operator can find the doc.
  designDoc: string;
};

const StubPage: Component<Props> = (props) => {
  return (
    <div class="flex h-full w-full items-center justify-center p-12">
      {/* Phase B Slice 7 — minimal hint set for stub pages. L1/R1
          inherit the shell-level tab cycling (wired in
          RetroverseShell). */}
      <HintRegion hints={{ l1: "Prev tab", r1: "Next tab" }} />
      <div class="max-w-md rounded-xl border border-white/10 bg-white/[0.03] px-10 py-12 text-center backdrop-blur">
        <p class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
          Retroverse UI · Phase B placeholder
        </p>
        <h1 class="mt-3 text-2xl font-semibold uppercase tracking-widest text-(--color-oa-ink)">
          {props.title}
        </h1>
        <p class="mt-4 text-sm text-(--color-oa-ink-dim)">
          This tab is coming in Phase C. The LIBRARY tab is the only
          surface implemented in Phase B; the other five render this
          placeholder so the top-tab routing is observably correct.
        </p>
        <p class="mt-6 font-mono text-[0.7rem] text-(--color-oa-ink-dim)">
          design doc:&nbsp;
          <span class="text-(--color-system-accent)">{props.designDoc}</span>
        </p>
      </div>
    </div>
  );
};

export default StubPage;
