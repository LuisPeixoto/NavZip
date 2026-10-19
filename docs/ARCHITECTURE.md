# Architecture

NavZip is a downstream Chromium browser with a focused patch and resource layer. Chromium remains responsible for rendering, JavaScript execution, navigation, profiles and browser services.

## Repository map

```text
chromium/
  engine.lock.json       Chromium and depot_tools pins
  config/                Development and distribution GN arguments
  patches/               Reviewable changes to upstream files
  overlay/               NavZip-owned native code and branding assets
webui/newtab/             New-tab HTML, CSS, TypeScript and production Mojo bridge
crates/navzip-extensions/ Package validation and bounded Wasm execution
crates/navzip-helper/     Legacy profile export and isolated plugin process
xtask/                    Rust build, packaging, capture and monitoring tools
tests/visual/             Component checks with a test-only bridge
tests/native/             Integration checks against the built browser
docs/                     Project handbook and release evidence
```

## Browser and interface

The full `chrome` target supplies Blink, V8, networking and native services. Window controls, the address bar and tabs use C++/Views. The new tab is a local `chrome://new-tab-page/` WebUI, not an iframe containing another browser. Vertical tabs and splits use Chromium's native controllers and WebContents.

The visible interface and page compositor remain part of Chromium's graphics pipeline. There is no continuous screenshot or pixel-copy bridge for browsing pages.

### Privileged boundary

`NavZipPreferences` and `NavZipShortcut` are typed Mojo structures. The trusted new-tab page can load/save shortcuts, change tab orientation and request a split. The browser validates sizes, unique IDs, HTTP/HTTPS schemes and absence of embedded credentials before saving atomically. Off-the-record saves are rejected. External pages cannot import this bridge.

The new tab starts without bundled sites. Reserved IDs from earlier bundled shortcuts are filtered when loading old preferences; user-created UUID shortcuts remain. Weather, city and personal-name fields and the native weather API have been removed. Icons are local; the clock pauses while the document is hidden.

### Upstream services

Bookmarks, history, passwords, permissions, cookies, cache, downloads, PDF, printing, Chromium extensions and off-the-record profiles use Chromium services. Sandbox, site isolation, TLS and certificate checks stay enabled. Retaining an upstream implementation does not establish NavZip release qualification for every flow.

## Rust boundaries

The workspace contains `xtask`, `navzip-helper` and `navzip-extensions`. The old Servo/egui application, its installers and its parallel state/platform crates were removed. Rust is used where it fits the project; the native browser is not presented as entirely Rust.

### Alpha profile migration

`migration/alpha.rs` contains data-only compatibility with Alpha v1. The exporter opens the original SQLite database read-only and uses SQLite backup to capture an active WAL consistently. It creates a private destination directory on Unix, refuses an existing backup directory and writes a versioned JSON document. It does not access the legacy password vault or export cookies/cache.

The C++ importer is explicitly enabled by `--navzip-import=PATH`. It reads off the UI thread, validates the envelope and writes through BookmarkModel/HistoryService on the appropriate sequence. Bookmark UUIDs and visit timestamps support repeatability. Crash recovery and complete native migration qualification remain pending.

### Experimental plugins

The Wasm helper uses fuel, memory/action limits, no WASI and a bounded standard-input/output protocol. The host must independently authorize each privileged action. This helper is tested separately; an installation/consent UI connected to Chromium is not yet available. See [Extensions](EXTENSIONS.md).

## Source and build integrity

`chromium/engine.lock.json` pins the engine and depot_tools. `cargo xtask apply` verifies the revision, checks patch application/reversal and copies the owned overlay and WebUI. The large upstream checkout, user profiles, build outputs and test logs are ignored by Git. See [Chromium workflow](CHROMIUM.md).
