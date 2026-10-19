export interface Shortcut { id: string; title: string; url: string; }
export interface Preferences { shortcuts: Shortcut[]; }
export interface Bridge {
  load(): Promise<Preferences>;
  save(prefs: Preferences): Promise<boolean>;
  vertical(enabled: boolean): Promise<boolean>;
  split(): Promise<boolean>;
}
export const defaults: Preferences = {shortcuts: []};
export function pageUrl(value: string): URL | null {
  try { const url = new URL(value); return ['https:', 'http:'].includes(url.protocol) && !url.username && !url.password ? url : null; } catch { return null; }
}
export function resolveAddress(value: string): string {
  const input = value.trim();
  const direct = pageUrl(input);
  if (direct) return direct.href;
  if (!/\s/.test(input) && /^(localhost|[\w-]+\.[\w.-]+)(:\d+)?(\/.*)?$/u.test(input)) {
    const candidate = pageUrl(`${input.startsWith('localhost') ? 'http' : 'https'}://${input}`);
    if (candidate) return candidate.href;
  }
  return `https://duckduckgo.com/?q=${encodeURIComponent(input)}`;
}
