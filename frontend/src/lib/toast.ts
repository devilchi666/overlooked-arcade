import { emit } from "@tauri-apps/api/event";

// Frontend toast push helpers. Mirror the Rust ToastPayload shape consumed
// by ToastStack.tsx via the `oa://toast` channel.
//
// Use `pushToast` for direct messages and `reportInvokeError` for the
// "user-initiated setter failed silently" shape — the latter logs to the
// console (devs see it) AND surfaces a toast (operators see it) so a
// rejected invoke doesn't vanish.

export type ToastLevel = "info" | "success" | "warn" | "error";

export function pushToast(level: ToastLevel, message: string, system?: string): void {
  void emit("oa://toast", { level, message, system }).catch((e) => {
    console.warn("[oa-toast] emit failed:", e);
  });
}

export function reportInvokeError(command: string, err: unknown): void {
  const msg = err instanceof Error ? err.message : String(err);
  console.warn(`[oa-invoke] ${command} failed:`, err);
  pushToast("error", `${command} failed: ${msg}`);
}
