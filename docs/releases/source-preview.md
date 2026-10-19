# NavZip 0.2.0-dev.1 — source preview

This prerelease makes the Chromium-based NavZip source available in versioned archives. **It contains no macOS, Windows or Linux browser installer.** The source ZIP/TAR.GZ is not a runnable application.

## Included

- Pinned Chromium patch/overlay and native NavZip branding.
- Minimal new tab, horizontal/vertical tabs, split views and matching light/dark themes.
- Rust build, migration, plugin and release-preparation tools.
- English handbook, screenshots, contribution policies and source/security workflows.
- Source archives, `SHA256SUMS` and a machine-readable `release-manifest.json` tied to the source commit and engine pin.

## Platform availability

| Platform | Status |
| --- | --- |
| macOS Intel | Development browser tested locally; standalone distribution and installer qualification pending |
| macOS Apple Silicon | Native browser build and qualification pending |
| Windows x64 | Native browser build, installer and qualification pending |
| Linux x64 | Native browser build, packages and qualification pending |

No dedicated Chromium build runners are registered yet. The existing native workflow defines these targets, but a workflow definition is not a built artifact. An old Servo Alpha DMG is not part of this release and must not be used as a substitute.

## Validation

The macOS Intel Chromium development build passed 16 local native integration checks. Public source CI checks Rust tooling on Linux/macOS/Windows, the WebUI component, workflow syntax and documentation integrity. Those checks do not establish browser support on every platform. C++ test objects were compiled but their test binary has not been executed.

## Build and verify

Follow the repository's [build guide](https://github.com/LuisPeixoto/NavZip/blob/main/docs/BUILDING.md). Chromium is fetched separately and requires its native toolchain and substantial disk space. Verify downloaded source files with `SHA256SUMS` before extracting them. Checksums identify files; they are not production code-signing signatures.

Qualified installers, code signing/notarization, signed updates, complete accessibility/security review and real-account web-app qualification remain pending. No performance advantage over Chrome or Brave is claimed. This preview is not recommended as a primary browser.
