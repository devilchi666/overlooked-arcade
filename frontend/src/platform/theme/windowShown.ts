// Window-presented signal (Theming ARC 3 M1). The Rust shell creates its
// window hidden and shows it only once the frontend reports first paint
// (`oa_shell_ready`) — or a timeout fallback fires — emitting `oa://window-shown`.
// App.tsx mirrors that event into this module-level signal; the `DeclarativeShell`
// keys its entrance view transition on it, so the motion plays exactly when the
// window is actually on screen (a play at component mount is masked — the window
// isn't composited yet — which is why the entrance was invisible before).
//
// Module-level (not context) so the cross-cutting App wiring and the theme shell
// share one source of truth without threading it through ThemeContext.

import { createSignal } from "solid-js";

const [windowShown, setWindowShown] = createSignal(false);

/// True once the shell window has been presented to the user. Reactive.
export { windowShown };

/// Mark the window as presented (called by App.tsx on the `oa://window-shown`
/// event). Idempotent — flipping an already-true signal is a no-op.
export function markWindowShown(): void {
  setWindowShown(true);
}
