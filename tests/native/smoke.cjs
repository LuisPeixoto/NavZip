// Exercises a built NavZip, with its real Mojo bridge and Chromium services.
// No installed-browser fallback, fake engine, sandbox bypass or user profile.
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const http = require('node:http');
const crypto = require('node:crypto');
const assert = require('node:assert/strict');
const {spawn, spawnSync} = require('node:child_process');
const {chromium} = require('playwright');
const root = path.resolve(__dirname, '../..');
const source = path.resolve(process.env.NAVZIP_CHROMIUM_ROOT || path.join(root, '.cache/chromium'), 'src');
const binary = process.env.NAVZIP_NATIVE_BINARY || path.join(source, 'out', process.env.NAVZIP_CHROMIUM_OUT || 'NavZip',
  process.platform === 'darwin' ? 'NavZip.app/Contents/MacOS/NavZip' : process.platform === 'win32' ? 'chrome.exe' : 'chrome');
const output = path.join(root, 'dist/qa-chromium/native');
const reportPath = path.join(output, 'report.json');
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));

async function until(check, description, milliseconds = 30000) {
  const end = Date.now() + milliseconds;
  while (Date.now() < end) {
    if (await check()) return;
    await delay(100);
  }
  throw new Error(`Tempo esgotado: ${description}`);
}

async function run() {
  fs.mkdirSync(output, {recursive: true});
  fs.writeFileSync(reportPath, JSON.stringify({status: 'running', startedAt: new Date().toISOString(), binary}, null, 2));
  assert(fs.statSync(binary, {throwIfNoEntry: false})?.isFile(),
    'Compile o Chromium/NavZip primeiro. Nenhum navegador instalado será aberto.');
  const video = path.join(source, 'media/test/data/bear-vp9.webm');
  const pdf = path.join(source, 'chrome/test/data/pdf/test.pdf');
  assert(fs.existsSync(video) && fs.existsSync(pdf), 'Fixtures upstream ausentes no checkout fixado.');
  const temporary = fs.mkdtempSync(path.join(os.tmpdir(), 'navzip-native-qa-'));
  const profile = path.join(temporary, 'profile');
  const extension = path.join(temporary, 'extension');
  fs.mkdirSync(extension, {recursive: true});
  fs.mkdirSync(output, {recursive: true});
  fs.writeFileSync(path.join(extension, 'manifest.json'), JSON.stringify({
    manifest_version: 3, name: 'NavZip integration fixture', version: '1.0',
    content_scripts: [{matches: ['http://127.0.0.1/*'], js: ['content.js'], run_at: 'document_end'}],
  }));
  fs.writeFileSync(path.join(extension, 'content.js'), 'document.documentElement.dataset.navzipExtension = "loaded";');
  const privateMarker = `private-${crypto.randomUUID()}`;
  const payload = Buffer.from('NavZip authenticated download fixture\n');
  const server = http.createServer((request, response) => {
    const url = new URL(request.url, 'http://127.0.0.1');
    response.setHeader('Cache-Control', 'no-store');
    if (url.pathname === '/video.webm' || url.pathname === '/document.pdf') {
      const file = url.pathname === '/video.webm' ? video : pdf;
      const bytes = fs.statSync(file).size;
      response.setHeader('Content-Type', url.pathname === '/video.webm' ? 'video/webm' : 'application/pdf');
      response.setHeader('Accept-Ranges', 'bytes');
      const range = /^bytes=(\d+)-(\d*)$/.exec(request.headers.range || '');
      const start = range ? Number(range[1]) : 0;
      const end = range?.[2] ? Math.min(Number(range[2]), bytes - 1) : bytes - 1;
      if (start > end || start >= bytes) { response.writeHead(416).end(); return; }
      if (range) {response.statusCode = 206; response.setHeader('Content-Range', `bytes ${start}-${end}/${bytes}`);}
      response.setHeader('Content-Length', end - start + 1);
      fs.createReadStream(file, {start, end}).pipe(response); return;
    }
    if (url.pathname === '/download') {
      if (!(request.headers.cookie || '').includes('navzip_login=normal')) {response.writeHead(401).end(); return;}
      response.setHeader('Content-Type', 'application/octet-stream');
      response.setHeader('Content-Disposition', 'attachment; filename="navzip-qa.txt"');
      response.end(payload); return;
    }
    response.setHeader('Content-Type', 'text/html; charset=utf-8');
    response.end('<!doctype html><title>NavZip native fixture</title><h1>Página real de teste</h1><button id="play" onclick="document.querySelector(\'video\').play()">Reproduzir</button><video src="/video.webm" controls></video><a id="download" href="/download">Baixar</a>');
  });
  let child, browser, cdp, closed = false;
  const childFinished = () => !child?.pid || child.exitCode !== null || child.signalCode !== null;
  const log = fs.openSync(path.join(output, 'browser.log'), 'w');
  const checks = [];
  function checkpoint(label) {
    checks.push(label);
    const previous = JSON.parse(fs.readFileSync(reportPath, 'utf8'));
    fs.writeFileSync(reportPath, JSON.stringify({...previous, checks,
      windowCaptures, captureLimitations, updatedAt: new Date().toISOString()}, null, 2));
    console.log(`OK · ${label}`);
  }
  const windowCaptures = [];
  const captureLimitations = [];
  function captureWindow(layout) {
    if (process.platform !== 'darwin') {
      captureLimitations.push(`${layout}: captura de moldura ainda não implementada nesta plataforma`);
      return;
    }
    const filename = path.join(output, `${layout}-window-${Date.now()}.png`);
    const capture = spawnSync('cargo', ['run', '--locked', '-p', 'xtask', '--bin', 'xtask', '--',
      'capture-window', String(child.pid), filename], {cwd: root, encoding: 'utf8', timeout: 60000});
    if (capture.status === 0) windowCaptures.push(filename);
    else captureLimitations.push(`${layout}: ${capture.error?.message || capture.stderr?.trim() || 'captura indisponível'}`);
  }
  try {
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    const local = `http://127.0.0.1:${server.address().port}`;
    child = spawn(binary, [`--user-data-dir=${profile}`, '--remote-debugging-port=0',
      '--remote-debugging-address=127.0.0.1', '--no-first-run', '--no-default-browser-check',
      '--lang=pt-BR', '--window-size=1612,934', `--load-extension=${extension}`, 'chrome://newtab/'],
      {stdio: ['ignore', log, log], detached: process.platform !== 'win32'});
    let launchError;
    child.on('error', error => {launchError = error;});
    const portFile = path.join(profile, 'DevToolsActivePort');
    await until(() => {
      if (launchError) throw launchError;
      if (childFinished()) throw new Error(`NavZip encerrou durante a inicialização: ${child.exitCode ?? child.signalCode}`);
      return fs.existsSync(portFile);
    }, 'inicialização do NavZip', 60000);
    const [port, endpoint] = fs.readFileSync(portFile, 'utf8').trim().split('\n');
    assert(/^\d+$/.test(port) && endpoint.startsWith('/devtools/browser/'));
    browser = await chromium.connectOverCDP(`ws://127.0.0.1:${port}${endpoint}`);
    cdp = await browser.newBrowserCDPSession();
    const downloads = path.join(temporary, 'downloads');
    fs.mkdirSync(downloads);
    await cdp.send('Browser.setDownloadBehavior', {behavior: 'allow', downloadPath: downloads, eventsEnabled: true});
    const context = browser.contexts()[0];
    const page = context.pages()[0] || await context.newPage();
    await page.waitForSelector('#clock');
    checkpoint('startup new tab loads without a navigation repair');
    await page.goto('chrome://new-tab-page/');
    await page.waitForSelector('.shortcut');
    assert.equal(await page.locator('#greeting, #weather, #name, #city, #weather-enabled').count(), 0);
    assert.equal(await page.locator('#shortcuts > a').count(), 0);
    const nativePrefs = await page.evaluate(async () => (await import('./navzip/navzip_bridge.js')).bridge.load());
    assert.deepEqual(nativePrefs, {shortcuts: []});
    const ownShortcut = {id: 'qa-custom', title: 'Meu site', url: 'https://example.org/'};
    for (const invalid of [
      {shortcuts: Array.from({length: 13}, (_, i) => ({...ownShortcut, id: `qa-${i}`}))},
      {shortcuts: [{...ownShortcut, title: 'x'.repeat(257)}]},
      {shortcuts: [ownShortcut, ownShortcut]},
      {shortcuts: [{...ownShortcut, url: 'javascript:alert(1)'}]},
      {shortcuts: [{...ownShortcut, url: 'https://user:secret@example.com/'}]},
    ]) {
      assert.equal(await page.evaluate(async prefs =>
        (await import('./navzip/navzip_bridge.js')).bridge.save(prefs), invalid), false);
      assert.deepEqual(await page.evaluate(async () =>
        (await import('./navzip/navzip_bridge.js')).bridge.load()), nativePrefs,
      'Preferências inválidas alteraram dados que deveriam permanecer intactos');
    }
    checkpoint('native Mojo rejects invalid preferences atomically');
    assert.equal(await page.evaluate(async shortcut => {
      const {bridge} = await import('./navzip/navzip_bridge.js');
      return bridge.save({shortcuts: [shortcut]});
    }, ownShortcut), true);
    await page.reload();
    await page.waitForSelector('a[href="https://example.org/"]');
    assert((await page.locator('#shortcuts > a').textContent()).includes('Meu site'));
    assert.equal(await page.evaluate(async prefs =>
      (await import('./navzip/navzip_bridge.js')).bridge.save(prefs), nativePrefs), true);
    await page.reload();
    await page.waitForSelector('.shortcut');
    assert.equal(await page.locator('#shortcuts > a').count(), 0);
    await page.bringToFront();
    await page.locator('#clock').click();
    await page.screenshot({path: path.join(output, 'newtab-real-content.png')});
    if (process.platform === 'darwin') {
      const viewport = await page.evaluate(() => ({inner: innerHeight, outer: outerHeight}));
      assert(viewport.outer - viewport.inner <= 100,
        `Cabeçalho horizontal ocupa espaço excessivo: ${JSON.stringify(viewport)}`);
    }
    captureWindow('horizontal-single');
    checkpoint('real Mojo preferences and persistence');
    const singleHeight = await page.evaluate(() => innerHeight);
    const extra = await context.newPage();
    await extra.goto(`${local}/second-tab`);
    await page.bringToFront();
    await until(async () => (await page.evaluate(() => innerHeight)) <= singleHeight - 30,
      'barra horizontal ao abrir a segunda aba');
    const multiHeight = await page.evaluate(() => innerHeight);
    assert(singleHeight - multiHeight <= 60, 'Mais de uma linha de abas foi reservada');
    await page.locator('#clock').click();
    captureWindow('horizontal');
    // Closing a background tab must also reclaim the tab-row space.
    await extra.close();
    await until(async () => (await page.evaluate(() => innerHeight)) === singleHeight,
      'ocultar barra horizontal ao voltar para uma aba');
    checkpoint('horizontal tabs automatically show/hide across 1 → 2 → 1 tabs');
    assert.equal(await page.evaluate(async () => (await import('./navzip/navzip_bridge.js')).bridge.vertical(true)), true);
    const singleWidth = await page.evaluate(() => innerWidth);
    const verticalExtra = await context.newPage();
    await verticalExtra.goto(`${local}/vertical-second-tab`);
    await page.bringToFront();
    await until(async () => (await page.evaluate(() => innerWidth)) <= singleWidth - 24,
      'barra vertical ao abrir a segunda aba');
    await verticalExtra.close();
    await until(async () => (await page.evaluate(() => innerWidth)) === singleWidth,
      'ocultar barra vertical ao voltar para uma aba');
    checkpoint('vertical tabs automatically show/hide across 1 → 2 → 1 tabs');
    const newPage = context.waitForEvent('page');
    newPage.catch(() => {}); // Observe rejection immediately if a command closes the browser.
    assert.equal(await page.evaluate(async () => (await import('./navzip/navzip_bridge.js')).bridge.split()), true);
    const right = await newPage;
    await right.goto(`${local}/right`);
    await until(async () => (await page.evaluate(() => innerWidth)) < 1000 &&
      (await right.evaluate(() => innerWidth)) < 1000, 'duas páginas na divisão');
    captureWindow('vertical-split');
    await page.goto(`${local}/left`);
    await page.locator('h1').click();
    await page.waitForFunction(() => document.documentElement.dataset.navzipExtension === 'loaded');
    assert.equal(await page.evaluate(async () => {
      try { await import('chrome://new-tab-page/navzip/navzip_bridge.js'); return true; }
      catch { return false; }
    }), false, 'Página externa obteve a ponte privilegiada do WebUI');
    checkpoint('external page cannot import the privileged WebUI bridge');
    checkpoint('native vertical controller, split WebContents and unpacked MV3 content script');
    await page.locator('#play').click();
    await page.waitForFunction(() => document.querySelector('video').currentTime > 0.2);
    checkpoint('non-DRM VP9 video playback');
    await page.evaluate(() => {document.cookie = 'navzip_login=normal; SameSite=Lax; path=/';});
    const downloadEvent = page.waitForEvent('download');
    downloadEvent.catch(() => {});
    await page.locator('#download').click();
    const download = await downloadEvent;
    assert.equal(await download.failure(), null);
    const downloadPath = path.join(output, 'authenticated-download.txt');
    // CDP explicitly redirects downloads, so Playwright's own artifact folder
    // is not the destination. Verify the actual native download before copying.
    assert.equal(download.suggestedFilename(), 'navzip-qa.txt');
    const nativeDownload = path.join(downloads, 'navzip-qa.txt');
    await until(() => fs.existsSync(nativeDownload), 'arquivo baixado pelo Chromium');
    fs.copyFileSync(nativeDownload, downloadPath);
    assert.deepEqual(fs.readFileSync(downloadPath), payload);
    checkpoint('cookie-authenticated download');
    const privateContext = await browser.newContext();
    const privatePage = await privateContext.newPage();
    await privatePage.goto(`${local}/${privateMarker}`);
    assert.equal(await privatePage.evaluate(() => document.cookie), '');
    assert.equal(await privatePage.evaluate(() => document.documentElement.dataset.navzipExtension), undefined);
    await privatePage.evaluate(() => {document.cookie = 'navzip_login=private; path=/';});
    assert((await page.evaluate(() => document.cookie)).includes('navzip_login=normal'));
    await privateContext.close();
    checkpoint('off-the-record cookie isolation');
    const httpsPage = await context.newPage();
    const secure = await httpsPage.goto('https://example.com/', {waitUntil: 'domcontentloaded'});
    assert(secure?.ok());
    const tls = await secure.securityDetails();
    // CDP reports HTTP/3 security as QUIC; its handshake uses TLS 1.3.
    assert(tls?.protocol?.startsWith('TLS') || tls?.protocol === 'QUIC',
      `Detalhes HTTPS inesperados: ${JSON.stringify(tls)}`);
    const now = Date.now() / 1000;
    assert(tls.issuer && tls.validFrom <= now && tls.validTo > now);
    checkpoint(`HTTPS with certificate verification enabled (${tls.protocol})`);
    const pdfErrors = [];
    right.on('pageerror', error => pdfErrors.push(String(error)));
    await right.goto(`${local}/document.pdf`);
    try {
      await until(async () => {
        for (const frame of right.frames()) {
          if (await frame.evaluate(() =>
            document.querySelector('pdf-viewer')?.loadProgress_ === 100).catch(() => false)) return true;
        }
        return false;
      }, 'PDF nativo completamente carregado', 15000);
    } catch (error) {
      fs.writeFileSync(path.join(output, 'pdf-diagnostic.json'), JSON.stringify({
        url: right.url(), errors: pdfErrors,
        frames: await Promise.all(right.frames().map(async frame => ({
          url: frame.url(), html: (await frame.content()).slice(0, 24000),
          state: await frame.evaluate(() => {
            const viewer = document.querySelector('pdf-viewer');
            return {progress: viewer?.loadProgress_, shadow: viewer?.shadowRoot?.innerHTML.slice(0, 12000)};
          }),
        }))),
      }, null, 2));
      await right.screenshot({path: path.join(output, 'pdf-native.png')});
      throw error;
    }
    await right.screenshot({path: path.join(output, 'pdf-native.png')});
    checkpoint('native PDF viewer fully loaded (print UI still requires visual review)');
    const themePage = await context.newPage();
    await themePage.goto('chrome://new-tab-page/');
    await themePage.waitForSelector('#clock');
    assert.equal(await themePage.evaluate(async () =>
      (await import('./navzip/navzip_bridge.js')).bridge.vertical(false)), true);
    const settings = await context.newPage();
    await settings.goto('chrome://settings/help');
    await settings.waitForSelector('settings-about-page');
    const branding = await settings.evaluate(async () => {
      const {loadTimeData} = await import('chrome://resources/js/load_time_data.js');
      return Object.fromEntries(['aboutPageTitle', 'aboutProductTitle',
        'defaultBrowserMakeDefault', 'aboutProductCopyright'].map(key =>
        [key, loadTimeData.getString(key)]));
    });
    for (const key of ['aboutPageTitle', 'aboutProductTitle', 'defaultBrowserMakeDefault']) {
      assert(branding[key].includes('NavZip'), `Identidade incorreta: ${key}: ${branding[key]}`);
      assert(!branding[key].includes('Chromium'), `Marca antiga em ${key}`);
    }
    assert(branding.aboutProductCopyright.includes('Chromium'), 'Atribuição upstream removida');
    assert((await settings.locator('cr-toolbar').first().getAttribute('page-name')).includes('NavZip'));
    checkpoint('localized NavZip product identity with upstream attribution preserved');
    await settings.goto('chrome://settings/appearance');
    const scheme = settings.locator('#colorSchemeModeSelect');
    await scheme.waitFor({state: 'visible'});
    async function primaryColor() {
      return settings.evaluate(() => {
        const probe = document.createElement('span');
        probe.style.color = 'var(--color-sys-primary)';
        document.body.append(probe);
        const color = getComputedStyle(probe).color;
        probe.remove();
        return color;
      });
    }
    await scheme.selectOption('1');
    await until(async () => await primaryColor() === 'rgb(22, 105, 74)', 'paleta NavZip clara nas configurações');
    const toolbarTitle = settings.locator('cr-toolbar').first().locator('h1');
    assert.equal(await toolbarTitle.evaluate(node => getComputedStyle(node).color), 'rgb(28, 41, 36)');
    await settings.locator('cr-toolbar').first().click({position: {x: 150, y: 20}});
    captureWindow('settings-light');
    await themePage.bringToFront();
    await until(async () => await themePage.evaluate(() =>
      document.documentElement.dataset.colorScheme === 'light' &&
      getComputedStyle(document.documentElement).color === 'rgb(28, 41, 36)'),
      'modo claro na nova aba');
    await themePage.locator('#clock').click();
    captureWindow('horizontal-light');
    await settings.bringToFront();
    await scheme.selectOption('2');
    await until(async () => await primaryColor() === 'rgb(140, 219, 182)', 'paleta NavZip escura nas configurações');
    await until(async () => await toolbarTitle.evaluate(node => getComputedStyle(node).color) === 'rgb(242, 242, 238)',
      'contraste do título NavZip no modo escuro');
    await settings.locator('cr-toolbar').first().click({position: {x: 150, y: 20}});
    captureWindow('settings-dark');
    await themePage.bringToFront();
    await until(async () => await themePage.evaluate(() =>
      document.documentElement.dataset.colorScheme === 'dark' &&
      getComputedStyle(document.documentElement).color === 'rgb(244, 242, 237)'),
      'modo escuro na nova aba por preferência do perfil');
    await themePage.locator('#clock').click();
    captureWindow('horizontal-dark');
    checkpoint('native appearance setting applies NavZip light and dark palettes to Settings and new tabs');
    await themePage.goto('chrome://newtab/');
    await themePage.waitForSelector('.shortcut');
    assert.equal(await themePage.locator('#greeting, #weather, #name, #city, #weather-enabled').count(), 0);
    assert.equal(await themePage.locator('#shortcuts > a').count(), 0);
    assert.equal(await themePage.evaluate(async () =>
      typeof (await import('./navzip/navzip_bridge.js')).bridge.weather), 'undefined');
    checkpoint('empty new tab persists without greeting, location or weather service');
    await cdp.send('Browser.close').catch(() => {});
    await until(childFinished, 'encerramento do navegador');
    closed = true;
    assert.equal(child.exitCode, 0, 'NavZip não encerrou normalmente');
    // Only the newly created QA profile is scanned, never the user's data.
    function inspect(directory) {
      for (const entry of fs.readdirSync(directory, {withFileTypes: true})) {
        const filename = path.join(directory, entry.name);
        if (entry.isDirectory()) inspect(filename);
        else if (entry.isFile()) {
          const data = fs.readFileSync(filename);
          assert(!data.includes(Buffer.from(privateMarker)), `Registro privado persistido em ${path.relative(profile, filename)}`);
        }
      }
    }
    inspect(profile);
    const diagnostics = fs.readFileSync(path.join(output, 'browser.log'));
    assert(!/bad Mojo message|bad IPC message|Incorrect scheme/.test(diagnostics.toString('utf8')),
      'O navegador registrou uma mensagem IPC inválida durante os fluxos nativos');
    assert(!diagnostics.includes(Buffer.from(privateMarker)),
      'Endereço privado encontrado no log de diagnóstico do navegador');
    checkpoint('no private navigation marker in the closed QA profile or browser diagnostic log');
    fs.writeFileSync(reportPath, JSON.stringify({
      status: 'passed', completedAt: new Date().toISOString(), binary, platform: process.platform,
      checks, scope: 'experimental integration smoke; not full release certification',
      fullNativeWindowCaptured: ['horizontal-single', 'horizontal', 'vertical-split'].every(layout =>
        windowCaptures.some(filename => path.basename(filename).startsWith(`${layout}-window-`))),
      windowCaptures, captureLimitations,
      sandboxDisabled: false, certificateChecksDisabled: false,
    }, null, 2));
    console.log(`NavZip native smoke: ${checks.length} verificações aprovadas, ${windowCaptures.length} capturas de janela. Comparação visual e critérios de release ainda exigem homologação.`);
  } finally {
    if (!closed && child?.pid) {
      if (process.platform !== 'win32') {
        try {process.kill(-child.pid, 'SIGTERM');} catch {}
      } else child.kill();
      await until(childFinished, 'limpeza do processo de teste', 10000).catch(() => {});
    }
    await browser?.close().catch(() => {});
    fs.closeSync(log);
    server.closeAllConnections();
    await new Promise(resolve => server.close(resolve));
    if (closed || childFinished()) fs.rmSync(temporary, {recursive: true, force: true});
    else console.error(`Perfil de QA preservado porque o processo ainda está ativo: ${temporary}`);
  }
}
run().catch(error => {
  console.error(error);
  try {
    const previous = JSON.parse(fs.readFileSync(reportPath, 'utf8'));
    fs.writeFileSync(reportPath, JSON.stringify({...previous, status: 'failed',
      completedAt: new Date().toISOString(), error: String(error)}, null, 2));
  } catch {}
  process.exitCode = 1;
});
