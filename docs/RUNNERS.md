# Native build runners

Full Chromium builds require trusted, dedicated hosts. No runners are currently registered for this repository; hosted source CI is not a replacement for native browser builds.

## When no native hosts are available

As checked on October 19, 2026, GitHub documents **14 GB SSD storage** for standard public Linux, Windows and macOS runners. Chromium's upstream [Linux](https://chromium.googlesource.com/chromium/src/+/main/docs/linux/build_instructions.md) and [Windows](https://chromium.googlesource.com/chromium/src/+/main/docs/windows_build_instructions.md) instructions call for at least **100 GB free disk space**. The standard hosted configuration does not meet that requirement. See [GitHub's runner specifications](https://docs.github.com/en/actions/reference/runners/github-hosted-runners) for current limits.

Source checks and source previews can run there. Full browser builds need a sufficiently provisioned host or an explicitly authorized paid build environment. Larger GitHub runners have additional plan and billing requirements; they are not provisioned by this repository's workflows. Do not dispatch unavailable self-hosted targets or describe their queued jobs as builds in progress.

The maintainer's macOS Intel workstation can build locally with `cargo xtask build --release`, without registering it as a persistent GitHub runner. This does not provide Windows, Linux or Apple Silicon qualification. No installer for those targets is published until the corresponding build and packaging checks actually pass.

## Target labels

| Host | Required labels |
| --- | --- |
| macOS Intel | `self-hosted`, `navzip-chromium`, `macos`, `x64` |
| macOS Apple Silicon | `self-hosted`, `navzip-chromium`, `macos`, `arm64` |
| Windows x64 | `self-hosted`, `navzip-chromium`, `windows`, `x64` |
| Ubuntu 24.04 x64 | `self-hosted`, `navzip-chromium`, `linux`, `x64` |

Register each host through repository **Settings → Actions → Runners → New self-hosted runner**. Use GitHub's generated instructions on that host; never commit registration tokens or host credentials. The maintainer must supply and authorize the machines. Do not repurpose a personal workstation or grant access to another host without its owner's authorization.

## Provisioning

- Install Git, Rust/rustup, Node/npm, Python and the platform's Chromium toolchain from [Building](BUILDING.md).
- Ensure the initial sync has at least 120 GiB free and preserve the 20 GiB operational reserve. Builds may require more; plan capacity from actual configurations.
- Use isolated build accounts and disposable or carefully managed workspaces. Do not store personal browsing profiles or production credentials on runners.
- Restrict runner access to trusted maintainers. Never run untrusted fork/PR code on these machines.

## Run the matrix

The **Chromium native build** workflow is manual and restricted to `main`. Once all hosts are available, it fetches the pin, builds a non-component browser, checks headers and runs the native integration script with temporary profiles. Evidence is uploaded separately for each target. It does not yet package or publish browser installers.

Do not dispatch the whole matrix before its hosts exist: jobs would remain queued. Runner registration and successful jobs must be verified before recording a target as built. Native results also do not replace signing, clean-machine installer checks or release qualification.

## From builds to downloads

The macOS ad-hoc packager exists but still needs a completed non-component build and distribution tests. Windows/Linux installer integration is pending. Do not rename source archives or old Alpha artifacts into platform installers. Future release assets must come from the pinned source, include their licenses/checksums, pass artifact installation/launch checks and state signing status accurately.
