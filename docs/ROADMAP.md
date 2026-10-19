# Roadmap

Milestones describe exit criteria, not promised dates. Progress is recorded in issues, pull requests and [validation](VALIDATION.md).

## 1 · A reviewable foundation

- [x] Chromium pin, patch/overlay workflow and Rust development tools.
- [x] Working native macOS Intel development browser.
- [x] Minimal new tab, horizontal/vertical tabs, split views and matching themes.
- [x] Local Rust, component and native integration checks.
- [x] Public English handbook, source CI and contribution workflows.

## 2 · Reliable developer preview

- [ ] Repeat native builds on macOS ARM64, Windows x64 and Linux x64.
- [ ] Run C++ test binaries and expand tab/group/restoration regression coverage.
- [ ] Qualify the native Alpha-profile importer and recovery without duplication.
- [ ] Validate Chrome Web Store and extension permission/update flows.
- [ ] Improve application localization and unify new-tab search with the selected provider.
- [ ] Audit keyboard use, focus, high contrast and screen-reader behavior.

## 3 · Daily-use qualification

- [ ] Review isolation and sandboxing on each platform.
- [ ] Complete real-account web-app, password/autofill, printing and download tests.
- [ ] Connect own Wasm packages through explicit consent and host-side authorization.
- [ ] Publish reproducible comparison methodology and measured results.

## 4 · A release people can trust

- [ ] Non-component distributable builds and clean-machine installers.
- [ ] Production signing, macOS notarization and verified update artifacts.
- [ ] Upgrade, rollback and uninstall qualification.
- [ ] Distribution license inventory, release checklist and supported-version policy.

See [release gates](RELEASE.md) for the acceptance standard. More users are a reason to require stronger evidence, not to skip these steps.
