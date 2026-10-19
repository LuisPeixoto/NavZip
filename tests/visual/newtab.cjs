// Validates the actual WebUI component with a test-only Mojo adapter.
// This is not a browser build or an end-to-end Chromium integration test.
const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');
const assert = require('node:assert/strict');
const {execFileSync} = require('node:child_process');
const {chromium} = require('playwright');
const root = path.resolve(__dirname, '../..');
const output = path.join(root, '.cache/webui-qa');
fs.mkdirSync(output, {recursive: true});
fs.cpSync(path.join(root, 'webui/newtab'), output, {recursive: true});
fs.copyFileSync(path.join(__dirname, 'bridge.ts'), path.join(output, 'navzip_bridge.ts'));
execFileSync(process.execPath, [require.resolve('typescript/bin/tsc'), '--strict', '--noPropertyAccessFromIndexSignature', '--noUncheckedIndexedAccess', '--target', 'ES2022', '--module', 'ESNext', '--moduleResolution', 'bundler', '--lib', 'DOM,ES2022', '--outDir', output, path.join(output, 'navzip_newtab.ts'), path.join(output, 'types.ts'), path.join(output, 'navzip_bridge.ts')], {stdio: 'inherit'});
const mime = {'.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.html': 'text/html'};
const server = http.createServer((request, response) => {
  const url = new URL(request.url, 'http://localhost');
  const name = path.resolve(output, '.' + decodeURIComponent(url.pathname === '/' ? '/navzip.html' : url.pathname));
  if (!name.startsWith(output + path.sep) || !fs.existsSync(name) || !fs.statSync(name).isFile()) {response.writeHead(404).end(); return;}
  response.setHeader('Content-Type', mime[path.extname(name)] || 'application/octet-stream');
  fs.createReadStream(name).pipe(response);
});
(async () => {
  let browser;
  try {
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    const executablePath = process.env.NAVZIP_QA_BROWSER || (process.platform === 'darwin' ? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' : undefined);
    browser = await chromium.launch({headless: true, executablePath});
    const context = await browser.newContext({viewport: {width: 1586, height: 832}, deviceScaleFactor: 1, locale: 'pt-BR', timezoneId: 'America/Sao_Paulo', reducedMotion: 'reduce', colorScheme: 'dark'});
    const page = await context.newPage(); const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    let external = 0;
    await context.route('https://**/*', route => {external++; return route.abort();});
    await page.clock.install({time: new Date('2026-10-16T19:24:00Z')});
    const address = `http://127.0.0.1:${server.address().port}/`;
    await page.goto(address); await page.waitForSelector('.shortcut');
    assert.equal(await page.locator('#clock').textContent(), '16:24');
    await page.clock.fastForward(60_000);
    assert.equal(await page.locator('#clock').textContent(), '16:25');
    await page.clock.setFixedTime(new Date('2026-10-16T19:24:00Z'));
    assert.equal(await page.locator('#shortcuts > a').count(), 0);
    assert.equal(await page.locator('.shortcut').count(), 1, 'Only the Add control starts on the page');
    assert.equal(await page.locator('#greeting, #weather, #name, #city, #weather-enabled').count(), 0);
    assert.equal(external, 0, 'Empty new tabs must not request external content');
    // Old weather/name preferences must not reactivate removed features.
    await page.evaluate(() => localStorage.setItem('test-preferences', JSON.stringify({
      name: 'Antigo', city: 'Campos dos Goytacazes', weatherEnabled: true, shortcuts: [],
    })));
    await page.reload(); await page.waitForSelector('.shortcut');
    assert.equal(await page.locator('#greeting, #weather').count(), 0);
    assert.equal(external, 0);
    await page.getByRole('button', {name: 'Personalizar', exact: true}).click();
    assert.equal(await page.locator('#preferences input').count(), 0);
    await page.getByRole('button', {name: 'Cancelar', exact: true}).click();
    await page.getByRole('button', {name: 'Adicionar', exact: true}).click();
    await page.locator('#shortcut-title').fill('Inválido');
    await page.locator('#shortcut-url').fill('javascript:alert(1)');
    await page.locator('#shortcut-form button.primary').click();
    assert.match(await page.locator('#shortcut-error').textContent(), /HTTP/);
    await page.locator('#shortcut-cancel').click();
    for (const [title, url] of [['<img src=x onerror=alert(1)>', 'https://example.org/'], ['Meu site', 'https://example.com/']]) {
      await page.getByRole('button', {name: 'Adicionar', exact: true}).click();
      await page.locator('#shortcut-title').fill(title);
      await page.locator('#shortcut-url').fill(url);
      await page.locator('#shortcut-form button.primary').click();
      await page.waitForSelector(`a[href="${url}"]`);
    }
    await page.reload(); await page.waitForSelector('a[href="https://example.org/"]');
    assert.equal(await page.locator('#shortcuts > a').count(), 2);
    assert.equal(await page.locator('#shortcuts img[src=x]').count(), 0);
    await page.getByRole('button', {name: 'Personalizar', exact: true}).click();
    await page.getByRole('button', {name: 'Remover Meu site', exact: true}).click();
    await page.getByRole('button', {name: 'Cancelar', exact: true}).click();
    assert.equal(await page.locator('#shortcuts > a').count(), 2, 'Cancel preserves shortcuts');
    await page.getByRole('button', {name: 'Personalizar', exact: true}).click();
    await page.getByRole('button', {name: 'Mover Meu site para antes', exact: true}).click();
    await page.getByRole('button', {name: 'Salvar', exact: true}).click();
    await page.waitForFunction(() => document.querySelector('#shortcuts > a')?.textContent.includes('Meu site'));
    await page.getByRole('button', {name: 'Personalizar', exact: true}).click();
    await page.getByRole('button', {name: 'Abas na lateral', exact: true}).click();
    assert.equal(await page.evaluate(() => localStorage.getItem('test-vertical')), 'true');
    await page.getByRole('button', {name: 'Personalizar', exact: true}).click();
    await page.getByRole('button', {name: 'Remover Meu site', exact: true}).click();
    await page.getByRole('button', {name: 'Remover <img src=x onerror=alert(1)>', exact: true}).click();
    await page.getByRole('button', {name: 'Salvar', exact: true}).click();
    await page.waitForFunction(() => document.querySelectorAll('#shortcuts > a').length === 0);
    await page.reload(); await page.waitForSelector('.shortcut');
    assert.equal(await page.locator('#shortcuts > a').count(), 0, 'Deleting the last shortcut remains empty');
    fs.mkdirSync(path.join(root, 'dist/qa-chromium'), {recursive: true});
    await page.clock.runFor(50);
    await page.screenshot({path: path.join(root, 'dist/qa-chromium/newtab-component.png')});
    await page.setViewportSize({width: 1612, height: 852});
    await page.clock.runFor(50);
    await page.screenshot({path: path.join(root, 'dist/qa-chromium/newtab-refined-reference.png')});
    await page.emulateMedia({colorScheme: 'light'});
    await page.clock.runFor(50);
    await page.screenshot({path: path.join(root, 'dist/qa-chromium/newtab-light-component.png')});
    await page.emulateMedia({colorScheme: 'dark'});
    await page.setViewportSize({width: 620, height: 872});
    await page.waitForFunction(() => document.querySelector('#query').placeholder === 'Pesquisar na web');
    await page.clock.runFor(50);
    await page.screenshot({path: path.join(root, 'dist/qa-chromium/newtab-narrow-component.png')});
    assert.equal(external, 0, 'No weather or favicon service remains active');
    assert.deepEqual(errors, []);
    console.log('WebUI: TypeScript, relógio, tela inicial vazia, ausência de clima/saudação, atalhos próprios, persistência, cancelamento e proteção de texto: OK. Capturas apenas do componente.');
  } finally {await browser?.close(); await new Promise(resolve => server.close(resolve));}
})().catch(error => {console.error(error); process.exitCode = 1;});
