import {bridge} from './navzip_bridge.js';
import {defaults, pageUrl, resolveAddress} from './types.js';
import type {Preferences, Shortcut} from './types.js';

function element<T extends HTMLElement>(id: string): T {
  const node = document.getElementById(id);
  if (!node) throw new Error(`Elemento ausente: ${id}`);
  return node as T;
}
let preferences: Preferences = structuredClone(defaults);
let draft: Preferences = structuredClone(defaults);
const dialog = element<HTMLDialogElement>('preferences');
const shortcutDialog = element<HTMLDialogElement>('shortcut-dialog');
const query = element<HTMLInputElement>('query');
const compactLayout = matchMedia('(max-width: 850px)');
const systemTheme = matchMedia('(prefers-color-scheme: dark)');
function updateSystemTheme() {
  if (document.documentElement.dataset['colorSchemeSource'] !== 'native') {
    document.documentElement.dataset['colorScheme'] = systemTheme.matches ? 'dark' : 'light';
  }
}
systemTheme.addEventListener('change', updateSystemTheme);
updateSystemTheme();
function updateSearchHint() {
  query.placeholder = compactLayout.matches ? 'Pesquisar na web' : 'O que vamos explorar hoje?';
}
compactLayout.addEventListener('change', updateSearchHint);
updateSearchHint();
const icons: Record<string, string> = {'github.com': 'github', 'www.youtube.com': 'youtube', 'youtube.com': 'youtube', 'figma.com': 'figma', 'www.figma.com': 'figma', 'mail.google.com': 'gmail', 'chatgpt.com': 'chatgpt'};

function notice(message: string) { element('status').textContent = message; }
function tick() {
  const date = new Date();
  const time = element<HTMLTimeElement>('clock');
  time.dateTime = date.toISOString();
  time.textContent = date.toLocaleTimeString('pt-BR', {hour: '2-digit', minute: '2-digit'});

}
let clockTimer: ReturnType<typeof setTimeout> | undefined;
function scheduleClock() {
  if (clockTimer) clearTimeout(clockTimer);
  if (!document.hidden) {
    tick();
    clockTimer = setTimeout(scheduleClock, 60_000 - Date.now() % 60_000);
  }
}
document.addEventListener('visibilitychange', scheduleClock);

function renderShortcuts() {
  const list = element('shortcuts'); list.replaceChildren();
  for (const shortcut of preferences.shortcuts) {
    const url = pageUrl(shortcut.url); if (!url) continue;
    const link = document.createElement('a'); link.className = 'shortcut'; link.href = url.href;
    const tile = document.createElement('span'); tile.className = 'shortcut-icon';
    const icon = icons[url.hostname];
    if (icon) { const image = document.createElement('img'); image.src = new URL(`icons/${icon}.svg`, import.meta.url).href; image.alt = ''; tile.append(image); }
    else { tile.textContent = shortcut.title.slice(0, 1).toLocaleUpperCase('pt-BR'); tile.style.fontSize = '30px'; }
    const title = document.createElement('span'); title.textContent = shortcut.title;
    link.append(tile, title); list.append(link);
  }
  if (preferences.shortcuts.length < 12) {
    const add = document.createElement('button'); add.className = 'shortcut'; add.ariaLabel = 'Adicionar';
    const tile = document.createElement('span'); tile.className = 'shortcut-icon'; tile.textContent = '+'; tile.style.fontSize = '42px'; tile.style.fontWeight = '200';
    const label = document.createElement('span'); label.textContent = 'Adicionar'; add.append(tile, label);
    add.addEventListener('click', () => { element<HTMLFormElement>('shortcut-form').reset(); element('shortcut-error').textContent = ''; shortcutDialog.showModal(); }); list.append(add);
  }
}
function renderShortcutSettings() {
  const container = element('shortcut-settings'); container.replaceChildren();
  draft.shortcuts.forEach((shortcut: Shortcut, index: number) => {
    const row = document.createElement('div'); row.className = 'shortcut-settings';
    const name = document.createElement('span'); name.textContent = shortcut.title;
    const up = document.createElement('button'); up.type = 'button'; up.textContent = '↑'; up.disabled = index === 0; up.ariaLabel = `Mover ${shortcut.title} para antes`;
    up.addEventListener('click', () => { const current = draft.shortcuts[index]; const previous = draft.shortcuts[index - 1]; if (current && previous) { draft.shortcuts[index] = previous; draft.shortcuts[index - 1] = current; renderShortcutSettings(); } });
    const remove = document.createElement('button'); remove.type = 'button'; remove.textContent = 'Remover'; remove.ariaLabel = `Remover ${shortcut.title}`;
    remove.addEventListener('click', () => { draft.shortcuts.splice(index, 1); renderShortcutSettings(); }); row.append(name, up, remove); container.append(row);
  });
}
element('customize').addEventListener('click', () => {
  draft = structuredClone(preferences);
  element('preferences-error').textContent = ''; renderShortcutSettings(); dialog.showModal();
});
element('cancel').addEventListener('click', () => dialog.close());
element('shortcut-cancel').addEventListener('click', () => shortcutDialog.close());
element('search').addEventListener('submit', event => { event.preventDefault(); if (query.value.trim()) location.assign(resolveAddress(query.value)); });
document.addEventListener('keydown', event => {
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') { event.preventDefault(); query.focus(); query.select(); }
});
if (!/Mac/i.test(navigator.platform)) element('shortcut-key').textContent = 'Ctrl K';
element('preferences-form').addEventListener('submit', async event => {
  event.preventDefault();
  const next = structuredClone(draft);
  try {
    if (!await bridge.save(next)) throw new Error('Não foi possível salvar estas preferências.');
    preferences = next; renderShortcuts(); dialog.close();
  } catch { element('preferences-error').textContent = 'Não foi possível salvar. Suas preferências anteriores foram preservadas.'; }
});
element('shortcut-form').addEventListener('submit', async event => {
  event.preventDefault();
  const title = element<HTMLInputElement>('shortcut-title').value.trim(); const url = pageUrl(element<HTMLInputElement>('shortcut-url').value.trim());
  if (!title || !url || preferences.shortcuts.length >= 12) { element('shortcut-error').textContent = 'Use um nome e um endereço HTTP ou HTTPS válido.'; return; }
  const next = {...preferences, shortcuts: [...preferences.shortcuts, {id: crypto.randomUUID(), title, url: url.href}]};
  try { if (!await bridge.save(next)) throw new Error('rejeitado'); preferences = next; renderShortcuts(); shortcutDialog.close(); }
  catch { element('shortcut-error').textContent = 'Não foi possível salvar o atalho.'; }
});
for (const [id, enabled] of [['horizontal', false], ['vertical', true]] as const) {
  element(id).addEventListener('click', async () => { try { if (!await bridge.vertical(enabled)) throw new Error('sem janela'); dialog.close(); } catch { element('preferences-error').textContent = 'Não foi possível alterar a posição das abas.'; } });
}
element('split').addEventListener('click', async () => { try { if (!await bridge.split()) throw new Error('sem janela'); dialog.close(); } catch { element('preferences-error').textContent = 'Selecione uma aba que ainda não esteja dividida.'; } });

async function start() {
  try { preferences = await bridge.load(); }
  catch { notice('Personalização indisponível. Tente abrir uma nova aba.'); }
  scheduleClock(); renderShortcuts();
}
void start();
