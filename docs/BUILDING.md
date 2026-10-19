# Building NavZip

NavZip builds the full Chromium `chrome` target. This is a native browser build, not an Electron app or a WebView wrapper.

## Requirements

| Requirement | Notes |
| --- | --- |
| Git, Rust/rustup, Python 3 | `rust-toolchain.toml` pins the Rust tools used by this repository |
| Node.js 22 or newer, npm | Component tests and native integration automation |
| Disk space | Initial sync requires 120 GiB free; a 20 GiB operational reserve is enforced during builds |
| Network access | Chromium, depot_tools, toolchains and dependencies are downloaded separately |
| Native platform toolchain | See the platform table below |

The free-space thresholds are safeguards, not a promise that every build fits in that amount. Tools stop instead of deleting your files to free space.

| Platform | Prerequisites | Browser qualification |
| --- | --- | --- |
| macOS Intel | Xcode, macOS SDK and Metal compiler | Native development build tested |
| macOS Apple Silicon | Xcode, matching SDK and native ARM64 environment | Pending |
| Windows x64 | Chromium-supported Visual Studio/MSVC and Windows SDK | Pending |
| Linux x64 | Initially Ubuntu 24.04 with Chromium build dependencies | Pending |

Use upstream's [macOS](https://chromium.googlesource.com/chromium/src/+/main/docs/mac_build_instructions.md), [Windows](https://chromium.googlesource.com/chromium/src/+/main/docs/windows_build_instructions.md) and [Linux](https://chromium.googlesource.com/chromium/src/+/main/docs/linux/build_instructions.md) requirements together with the pinned revision. Those pages track upstream; requirements may differ from the pin.

## Build and run

```sh
git clone https://github.com/LuisPeixoto/NavZip.git
cd NavZip
cargo xtask doctor
cargo xtask fetch
cargo xtask build
cargo xtask run -- --no-first-run
```

`fetch` retrieves the pinned source and dependencies and runs upstream hooks. `build` verifies the revision, applies NavZip patches and overlays, generates the build and compiles the browser. Re-running retains completed build work. Never update the checkout independently of `chromium/engine.lock.json`.

The default source root is `.cache/chromium/src`; development output is `out/NavZip` inside that source root. On macOS the app is `out/NavZip/NavZip.app`.

### Process-scoped configuration

| Variable | Purpose |
| --- | --- |
| `NAVZIP_CHROMIUM_ROOT` | Directory containing Chromium's `src/` checkout |
| `NAVZIP_PYTHON` | Python executable used by development tooling |
| `NAVZIP_BUILD_JOBS` | Local build parallelism; defaults to 6 |
| `DEVELOPER_DIR` | Xcode developer directory on macOS |
| `NAVZIP_QA_BROWSER` | Browser executable for component tests only |
| `NAVZIP_NATIVE_BINARY` | NavZip executable for native integration tests |

Example on macOS/Linux:

```sh
NAVZIP_BUILD_JOBS=6 NAVZIP_PYTHON=/path/to/python3 cargo xtask build
```

Use PowerShell's `$env:VARIABLE = 'value'` syntax on Windows. SDK/Xcode selection is scoped to build processes; tools do not change the global `xcode-select` setting.

### Metal on macOS

If Xcode distributes Metal separately, install it before building ANGLE:

```sh
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer xcodebuild -downloadComponent MetalToolchain
```

The validated local environment used Metal Toolchain 17F42. This is a recorded environment, not a universal requirement for other Chromium revisions.

## Check changes

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
npm ci --ignore-scripts
npx playwright install --with-deps chromium
npm test
```

`npm test` uses a test-only bridge and never proves that Chromium itself compiles. On macOS it defaults to installed Google Chrome; on Linux/Windows it uses Playwright Chromium. Both can be overridden with `NAVZIP_QA_BROWSER`.

For C++/Mojo or browser integration changes, also run:

```sh
cargo xtask build-integration
cargo xtask build
cargo xtask check
npm run test:native
```

`build-integration` compiles selected integration objects and resources, including C++ test objects; it does not execute those tests or link the full browser. Native tests launch only the built NavZip, with temporary profiles and no sandbox/certificate bypass. [Validation details](VALIDATION.md).

## Follow a long build

```sh
cargo xtask monitor
```

Open the loopback URL printed by the command. The read-only panel updates every two seconds from actual logs and pipeline state. Step counts may change; they are not time estimates.

On macOS, a build redirected to `.cache/chromium-native-build.log` can be followed by `cargo xtask wait-native BUILD_PID` while that exact build is running. It starts native tests only after a verified successful exit. It is a one-shot continuation, not a scheduler.

## Distribution builds

```sh
cargo xtask build --release
cargo xtask package
```

The default component build depends on libraries outside the app bundle. Packaging refuses it. Current macOS packaging requires a non-component build and produces an experimental ad-hoc DMG under `dist/chromium/`; production signing and notarization are not configured. Windows/Linux installers and signed updates remain pending.

## Common problems

- **Not enough disk space:** move the checkout or free space yourself, then rerun. No cleanup command deletes user data automatically.
- **Wrong revision / patch does not apply:** inspect the pin and local checkout diff. Preserve local work; do not use a blanket hard reset.
- **SDK or Metal missing:** run `cargo xtask doctor` and fix the native toolchain first.
- **`NavZip` is not built:** build the checkout. There is no fallback to installed Chrome, Servo, CEF or Electron.
- **Component test passes but browser fails:** run the native build, GN check and native integration script; they exercise different layers.
