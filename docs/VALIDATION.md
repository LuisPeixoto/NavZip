# Validation record

**Recorded on October 19, 2026. Scope: local macOS Intel development environment.** These results establish specific checks, not general security, performance or cross-platform claims. GitHub Actions results are separately visible in the repository.

## Evidence by layer

| Layer | Result | Boundary |
| --- | --- | --- |
| Rust workspace | 23 tests passed; Clippy with `-D warnings`; formatting passed | Tooling, migration and plugin components, not the engine |
| WebUI component | Strict TypeScript and Playwright checks passed | Test-only bridge in an installed browser |
| Chromium patch | Checked against the pinned upstream context | Does not establish compilation |
| Native build | Full development browser built on macOS Intel | Component build depends on checkout libraries |
| Header dependencies | GN check passed for 35,801 of 36,083 targets per upstream selection | Not a runtime test |
| Integration objects | Build passed, including updated C++ test objects and WebUI resources | C++ test binary not executed |
| Native integration | 16 checks passed; seven complete window captures | Local environment only; temporary test profiles |

The latest new-tab change built in 3 min 26 s; integration objects/resources built in 1 min 57 s. These are incremental local build timings, not benchmarks or estimates for a fresh checkout.

## Native integration checks

The latest local report completed at **2026-10-19 13:17:30 UTC**. The script launched the actual checkout executable, with sandbox and certificate checks enabled, and verified:

1. New-tab startup without a corrective navigation.
2. Atomic rejection of invalid Mojo preferences.
3. Shortcut preference persistence and return to an empty list.
4. Horizontal strip show/hide and recovered height across 1 → 2 → 1 tabs.
5. Vertical strip show/hide and recovered width across 1 → 2 → 1 tabs.
6. An external page cannot import the privileged WebUI bridge.
7. Native vertical controller, split WebContents and a local MV3 content script.
8. VP9 video playback without DRM.
9. Cookie-authenticated download with payload verification.
10. Off-the-record cookie isolation.
11. HTTPS with valid certificate verification; the observed protocol was QUIC/HTTP/3.
12. Fully loaded native PDF viewer; print UI remains unqualified.
13. Localized NavZip product text with upstream copyright attribution retained.
14. Native Settings and new-tab light/dark palette changes.
15. Empty new tab without greeting, location or weather service.
16. No private test marker in the closed QA profile or diagnostic log after normal shutdown.

The implementation in [`tests/native/smoke.cjs`](../tests/native/smoke.cjs) is the source of truth for assertions. The local report lives in `dist/qa-chromium/native/report.json`; generated logs and temporary profiles are intentionally not committed. Published screenshots are in the [gallery](SCREENSHOTS.md). A single absent marker does not prove complete absence of private records under all failure modes.

## Component and migration coverage

Component checks exercise an empty initial list, local clock updates, absence of removed greeting/weather controls, no external content requests, custom shortcut creation/reordering/deletion, persistence after reload, cancellation and safe text rendering. Their test adapter never enters the production Chromium resources.

Six migration tests reproduce the Alpha v1 SQLite schema directly. They cover JSON wire compatibility, settings and active-tab preservation, damaged-session normalization, rejected privileged/credential URLs, unknown versions and a consistent active-WAL backup without changing source database/WAL bytes. Native import idempotence and crash recovery still require qualification.

Plugin checks cover malformed packages, origin boundaries, changed consent, permission/private-policy rejection, foreign imports, memory growth, infinite execution and the real helper protocol.

## Visual evidence

The app has been captured with one/two horizontal tabs, a vertical split, Settings in both themes and the new tab in both themes. The maintainer's supplied screenshots show the later minimal page. Application clocks are real; only component tests control time. Review has been visual, without a pixel-equivalence claim across fonts or platforms.

The user's normal profile was reopened after the new-tab simplification, preserving its light-mode preference and existing data. The new page displayed no bundled sites, greeting or weather. No screenshots of confidential profiles or browsing histories are included in the repository.

## Resolved integration problems

- Text controls painted on transparent layers previously failed in `views::Label::PaintText`; custom labels now use grayscale antialiasing while Chromium checks remain enabled.
- The Material mixer previously overrode part of the NavZip palette; ordering now retains NavZip defaults while respecting custom themes and high contrast.
- A saved weather preference once triggered a forbidden renderer HTTPS request from privileged WebUI and `RESULT_CODE_KILLED_BAD_MESSAGE`. It was moved behind bounded native Mojo first; the subsequent simplification removed the feature and API entirely. No renderer network exception was added.
- Favicon masks now use transparent shapes, and the address focus outline matches the field's 4 px radius.
- The retired Servo application, installers and parallel state/platform code were removed. Only data-only migration compatibility remains.

## Still unvalidated

Native Windows/Linux/ARM64 browsers; production installers and updates; real-account Gmail/Docs/GitHub/Jira/Confluence/WhatsApp flows; broad extension compatibility; print UI; complete password/autofill flows; migration crash recovery; accessibility audits; platform isolation/security review; private-session crash cleanup; and reproducible performance comparisons.

Historical Servo logs and concept artwork do not validate this browser. See [release gates](RELEASE.md) before describing a build as ready for primary use.

## Source-preview preparation

Three additional Rust tests cover exact version/tag validation, clean-tree and conflicting-tag rejection, non-overwriting output, source archive generation and SHA-256 manifest verification. Prepared archives explicitly contain source, not platform installers. The source-publication workflow creates a draft only after required checks pass on its exact commit.
