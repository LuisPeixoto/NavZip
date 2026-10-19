# Working with Chromium

## Pinned base

The source of truth is [`chromium/engine.lock.json`](../chromium/engine.lock.json):

| Component | Pin |
| --- | --- |
| Chromium | `155.0.8059.80` |
| Source revision | `43a195105613ac83c21d3f14de279f4a045c7da0` |
| depot_tools | `251c84c1f36498529399517014e8f7994f762e2e` |

A fixed revision makes builds reviewable; it is not a guarantee that the browser is up to date or safe for primary browsing. Engine updates require patch review and repeated qualification. No automatic Chromium pin upgrade currently ships.

## Own the smallest practical delta

- **Patches:** changes to existing upstream files, with original attribution preserved.
- **Overlay:** NavZip-owned C++ code, icons and resources copied into the checkout.
- **WebUI:** files in `webui/newtab`, copied into Chromium resources; the production bridge uses generated Mojo types.
- **Checkout:** `.cache/chromium/src` by default. Never commit it, its `.git` metadata or output directories.

## Editing safely

1. Sync and confirm the pinned revision with `cargo xtask doctor`.
2. Inspect the current checkout diff before editing; preserve unrelated work.
3. Change owned files in this repository, or narrowly edit the relevant upstream files in the checkout.
4. Update the patch using only the intended upstream paths against the pinned base. Do not capture unrelated generated files or the entire checkout.
5. Validate a fresh upstream context with `cargo xtask overlay-check /path/to/clean/upstream-tree` when available. This checks patch context, not compilation.
6. Build integration objects and the full browser; run `cargo xtask check` and native tests.
7. Include the patch, owned resources, test changes and validation notes in one focused pull request.

`cargo xtask apply` recognizes already-applied patches through reverse checks. A mismatched context is an error, not permission to force-apply a patch or discard local work.

## Engine upgrades

Propose the version and revision together. Document security motivation, upstream API changes, patch conflicts, resource/license changes and validation by platform. Recheck new-tab Mojo boundaries, off-the-record isolation, extensions, installers and update recovery. A successful build is only one release gate.

## Legal boundaries

Preserve upstream copyright, license files and generated third-party notices. Do not replace technical attributions to Chromium with NavZip or import proprietary Chrome/Brave resources. See [third-party notices](../THIRD_PARTY_NOTICES.md).
