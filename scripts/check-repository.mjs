#!/usr/bin/env node
// Check owned repository metadata and local documentation links without downloads.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const files = [...new Set(execFileSync('git', ['ls-files', '--cached', '--others', '--exclude-standard', '-z'], {
  cwd: root, encoding: 'utf8', maxBuffer: 8 * 1024 * 1024,
}).split('\0').filter(Boolean))];
const problems = [];
const external = /^(?:https?:|mailto:|data:|#)/i;
function checkLink(file, target) {
  if (external.test(target)) return;
  const clean = decodeURIComponent(target.replace(/^<|>$/g, '').split('#')[0].split('?')[0]);
  if (!clean) return;
  const resolved = path.resolve(path.dirname(path.join(root, file)), clean);
  if (!resolved.startsWith(root + path.sep) || !fs.existsSync(resolved)) {
    problems.push(`${file}: missing or out-of-repository link: ${target}`);
  }
}
for (const file of files) {
  if (/(^|\/)(?:\.cache|target|node_modules|dist)(\/|$)/.test(file) ||
      /(^|\/)\.env(?:\.|$)/.test(file) || /\.(?:db(?:-wal|-shm)?|p12|pfx|pem|key)$/i.test(file)) {
    problems.push(`Generated or sensitive file must not be tracked: ${file}`);
  }
  const full = path.join(root, file);
  if (!fs.existsSync(full) || !fs.statSync(full).isFile()) continue;
  if (fs.statSync(full).size > 45 * 1024 * 1024) problems.push(`File exceeds reviewable size: ${file}`);
  if (file.endsWith('.json')) {
    try { JSON.parse(fs.readFileSync(full, 'utf8')); }
    catch (error) { problems.push(`${file}: ${error.message}`); }
  }
  if (!file.endsWith('.md')) continue;
  const content = fs.readFileSync(full, 'utf8').replace(/```[^\n]*\n[\s\S]*?```/g, '');
  for (const match of content.matchAll(/!?\[[^\]]*\]\(([^\s)]+)(?:\s+"[^"]*")?\)/g)) checkLink(file, match[1]);
  for (const match of content.matchAll(/(?:src|srcset|href)="([^"]+)"/g)) checkLink(file, match[1]);
}
const pin = JSON.parse(fs.readFileSync(path.join(root, 'chromium/engine.lock.json'), 'utf8'));
assert.match(pin.revision, /^[a-f0-9]{40}$/);
assert.match(pin.depot_revision, /^[a-f0-9]{40}$/);
assert.equal(pin.product, 'NavZip');
const toolchain = /channel\s*=\s*"([^"]+)"/.exec(fs.readFileSync(path.join(root, 'rust-toolchain.toml'), 'utf8'))?.[1];
for (const file of files.filter(file => /^\.github\/workflows\/.*\.ya?ml$/.test(file))) {
  const content = fs.readFileSync(path.join(root, file), 'utf8');
  for (const use of content.matchAll(/^\s*-?\s*uses:\s*([^\s#]+)/gm)) {
    if (!use[1].startsWith('./') && !/@[a-f0-9]{40}$/.test(use[1])) problems.push(`${file}: action must use a full commit SHA: ${use[1]}`);
  }
  for (const version of content.matchAll(/^\s*toolchain:\s*["']?([\d.]+)["']?\s*$/gm)) {
    if (version[1] !== toolchain) problems.push(`${file}: Rust pin differs from rust-toolchain.toml`);
  }
}
if (problems.length) {
  console.error(problems.join('\n'));
  process.exitCode = 1;
} else {
  console.log(`Repository checks passed: ${files.length} files, local documentation links, JSON, engine pins and action references.`);
}
