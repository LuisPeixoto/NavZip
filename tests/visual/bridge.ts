// Component test adapter. Never copied into the Chromium overlay or installers.
import {defaults} from './types.js';
import type {Bridge, Preferences} from './types.js';
export const bridge: Bridge = {
  async load() { const saved = JSON.parse(localStorage.getItem('test-preferences') || JSON.stringify(defaults)) as Preferences; return {shortcuts: saved.shortcuts}; },
  async save(prefs) { localStorage.setItem('test-preferences', JSON.stringify(prefs)); return true; },
  async vertical(enabled) { localStorage.setItem('test-vertical', String(enabled)); return true; },
  async split() { localStorage.setItem('test-split', 'true'); return true; },
};
