# Publishing previews and native releases

NavZip uses a single version per release, with platform assets attached only when actually built and validated. It does not create separate empty macOS/Windows/Linux releases to imply downloads exist.

## Source previews

The **Source preview release** workflow runs after successful main-branch source checks, or can be dispatched manually on `main`. It verifies required checks on the exact checked-out commit, stages a draft with source ZIP/TAR.GZ archives, SHA-256 checksums and a manifest, confirms four uploaded assets, then publishes it as a **source-only prerelease**. An existing version is skipped rather than overwritten. The automated path accepts only successful push checks from this repository on `main`, never untrusted pull-request/fork runs.

The input tag must match Cargo's version, currently `v0.2.0-dev.1`. Existing output directories, conflicting tags and uncommitted source are rejected; existing release assets are not overwritten.

For local preparation from a clean commit:

```sh
cargo xtask prepare-release v0.2.0-dev.1
```

Output: `dist/releases/v0.2.0-dev.1/`. This command does **not** compile the browser, create installers, sign code or publish anything. The manifest explicitly records `nativeInstallersIncluded: false`.

On Linux/macOS, verify the files from their download directory with `sha256sum --check SHA256SUMS` or `shasum -a 256 --check SHA256SUMS`. On Windows, use PowerShell `Get-FileHash -Algorithm SHA256` and compare each value with the checksum file.

## Native browser assets

Before publishing macOS, Windows or Linux downloads:

1. Provision the [dedicated native runners](RUNNERS.md).
2. Build and test each target from the pinned source and release commit.
3. Implement/qualify its standalone packaging, install, update and uninstall flows.
4. Include distribution-specific licenses and record hashes/provenance.
5. Complete signing/notarization and clean-machine checks; label experimental unsigned artifacts honestly if distributing them for developer testing.
6. Review all [release gates](RELEASE.md) before a stable label.

Current availability is recorded in [Getting started](GETTING_STARTED.md) and the release notes. Source previews are not platform binaries. Never attach the retired Servo DMG to the Chromium release line.
