# Release gates

**Current stage: experimental developer preview. No qualified Chromium release or signed installer.** Do not publish a stable label or download recommendation based only on source CI.

| Gate | Evidence / remaining work |
| --- | --- |
| Pinned source and dependencies | Chromium/depot_tools revisions verified locally |
| Native macOS Intel build | Development browser linked and exercised; not a standalone distribution |
| New tab and native frame | Captured in both themes and tab orientations; accessibility review pending |
| Horizontal/vertical tabs and splits | Native integration checks passed; complete reorder/pin/group/crash-restoration coverage pending |
| Migration without loss or duplication | Exporter tested, including active WAL; full C++ import and crash qualification pending |
| Private browsing and sandbox | Local cookie/marker checks passed; platform security review and crash-path tests pending |
| Chromium extensions | Local MV3 content script tested; Web Store and broader permission/update flows pending |
| Own Wasm extensions | Library/worker tested; browser UI and consent integration pending |
| Real-account web apps | Gmail, Docs, YouTube without DRM, GitHub, Jira/Confluence and WhatsApp Web need qualification |
| PDF and authenticated downloads | Native checks passed; print UI and complete password/autofill flows pending |
| Accessibility | Native mechanisms and semantic markup retained; keyboard/screen-reader audit pending |
| Other browser platforms | Windows x64, Linux x64 and macOS ARM64 builds/validation pending |
| Install / uninstall / update | Clean-machine tests and rollback pending |
| Signing and notarization | Production certificates, verification and update-signing path pending |
| Performance | Reproducible 1/10/30-tab startup, CPU and total-process-memory comparisons pending |
| Licenses and notices | Generate/review distribution-specific third-party inventory before publishing binaries |

## A release must be reviewable

1. Pin and review the engine, tools and NavZip source revision.
2. Record build configuration, platform, artifact hashes and relevant environment details.
3. Run source checks, native tests and the complete platform qualification matrix.
4. Review security, privacy, accessibility and known issues.
5. Package a non-component build; verify all bundled dependencies on a clean machine.
6. Sign/notarize as applicable and verify signatures before installation.
7. Exercise installation, uninstall, upgrade, interrupted upgrade and recovery.
8. Publish accurate release notes, source/license information and reproducible benchmark methodology.

Experimental ad-hoc builds must be labeled as such. No release workflow currently pretends to sign, notarize or upload a production-ready browser. Windows installer identities and COM integration also require review.

DRM, video calling, cloud synchronization and a NavZip account are outside the initial release scope.
