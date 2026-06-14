// The per-system domain set surfaced as cards inside a system's hub. `enabled`
// gates which are wired this slice (S1: Display & Core; the rest land in
// S2-S4 and render as "Coming soon" disabled cards meanwhile).

export type DomainId = "display" | "input" | "core" | "media" | "metadata" | "bios";

export type DomainDef = {
  id: DomainId;
  label: string;
  glyph: string;
  blurb: string;
  enabled: boolean;
};

export const DOMAINS: readonly DomainDef[] = [
  { id: "display", label: "Display & Video", glyph: "▣", blurb: "Scaling · shader · aspect · rewind", enabled: true },
  { id: "core", label: "Core / Launcher", glyph: "⚙", blurb: "Default core + external launcher", enabled: true },
  { id: "media", label: "Media", glyph: "⊞", blurb: "Art slots + game-media ops", enabled: true },
  { id: "metadata", label: "Metadata", glyph: "✎", blurb: "System facts + peripherals", enabled: true },
  { id: "input", label: "Input", glyph: "◉", blurb: "Bindings + core options", enabled: true },
  { id: "bios", label: "BIOS", glyph: "⚕", blurb: "Required BIOS status", enabled: true },
];

export function domainLabel(id: DomainId): string {
  return DOMAINS.find((d) => d.id === id)?.label ?? id;
}
