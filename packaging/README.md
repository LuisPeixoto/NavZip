# Distribution

Packaging is implemented in `xtask/src/chromium.rs` and operates only on the Chromium/NavZip build.

```sh
cargo xtask build --release
cargo xtask package
```

The macOS packager requires a non-component build and produces `dist/NavZip-<project-version>-macos-<architecture>.dmg`, an **experimental ad-hoc DMG**. The staged application is under `dist/chromium/`. It is not production-signed or notarized. Default component builds depend on checkout libraries and are rejected by the packager.

Release builds enable Chromium's generated credits. Packaging rejects missing/sample credits, includes upstream `LICENSE`, `AUTHORS` and generated credits alongside NavZip notices, verifies the ad-hoc bundle signature and checks the disk image with `hdiutil verify`. These checks do not replace a complete license review, a clean-machine launch test or production notarization.

Windows/Linux installers, production signing, notarization, clean-machine installation/uninstallation and signed updates remain unimplemented or unqualified. See the [release gates](../docs/RELEASE.md) before publishing binaries.

Old Servo Alpha installation templates were removed. Old artifacts on a developer's disk are not a release of the Chromium line. The end user of a future packaged release should not need Rust; building from source currently requires development tools.
