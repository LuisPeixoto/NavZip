# Frequently asked questions

## Can I use NavZip as my main browser?

Not yet. This is an experimental developer preview with a working macOS Intel build and explicit [release gates](RELEASE.md). Qualified installers, an update channel, platform validation and security review are still pending.

## Is this Chrome, Electron or a WebView?

NavZip builds the full open-source Chromium browser. It uses Blink and V8, native browser services and its own patch/resource layer. It does not open installed Chrome as its application engine. Component tests may use an installed browser; native tests use the NavZip executable exclusively.

## Is everything written in Rust?

No. Chromium's browser and engine include C++ and other upstream languages. NavZip uses C++/Views for native UI, HTML/CSS/TypeScript and Mojo for the new tab, and Rust for tooling, migration and a bounded plugin worker. The retired Servo application is not distributed by this line.

## Where is the download?

There is no qualified Chromium installer to recommend yet. Watch [Releases](https://github.com/LuisPeixoto/NavZip/releases). An old Alpha DMG or installed app is not the Chromium build. A development component build also depends on its checkout.

## Are Windows and Linux supported?

They are intended native build targets, along with macOS Intel and Apple Silicon. The current browser evidence is macOS Intel only. Hosted Rust CI on multiple OSes does not prove that the full browser works there.

## Why are the screenshots in Portuguese?

The initial custom UI uses Brazilian Portuguese. Documentation, commits and contribution workflows use English so international contributors can participate. Complete application localization is future work; do not infer it from the documentation language.

## Why does the tab bar disappear?

NavZip hides it while one tab is open. Command+T / Ctrl+T opens another tab and reveals it. The orientation setting persists when the strip is hidden.

## How do I add sites to the new tab?

Choose **Adicionar** (Add), enter a title and HTTP/HTTPS URL, then save. Use **Personalizar** (Customize) to remove or reorder shortcuts. Empty remains empty after reload; NavZip does not refill it with suggestions.

## Which search engine is used?

The address bar uses Chromium's search settings. Text searches in the custom new-tab field currently use DuckDuckGo; direct HTTP/HTTPS addresses navigate directly. Unifying that field with the selected browser search provider is future work. Typing is not a claim that suggestions or search traffic are fully audited.

## Do Chrome extensions work?

Chromium's extension services are retained and a local MV3 content script is covered by native tests. Chrome Web Store installation, updates and broad compatibility are not qualified yet. The separate `.navzip-extension` Wasm format has a tested library/worker but no integrated browser installation UI. See [Extensions](EXTENSIONS.md).

## Does private browsing make me anonymous?

No. It uses Chromium's off-the-record profile. Cookie isolation and absence of a test marker after a normal close have been checked locally; this is not a complete privacy audit or anonymity guarantee.

## Is it faster or lighter than Chrome or Brave?

That has not been established. Reproducible startup, CPU and total-process-memory benchmarks at 1, 10 and 30 tabs are release work. Using Rust in supporting tools does not establish a browser performance advantage.

## Is NavZip affiliated with Google or Brave?

No. Upstream components retain their licenses and attribution. NavZip's own code is MPL-2.0; see [third-party notices](../THIRD_PARTY_NOTICES.md).
