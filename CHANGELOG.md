# Changelog

Notable changes are recorded here. Versions and dates describe source milestones, not automatically qualified binary releases.

## Unreleased — Chromium developer preview

### Added

- Pinned Chromium 155.0.8059.80 with a reviewable patch/overlay layer.
- Native NavZip branding, graphite/mint dark mode and green-accented light mode.
- Horizontal and vertical tabs, automatic single-tab strip hiding and native split views.
- Minimal new tab with local clock, search and user-created shortcuts.
- Rust build/monitoring tools, consistent Alpha-profile export and bounded Wasm helper.
- Source/component checks and native integration automation.
- English handbook, actual screenshots, labeled design concepts and contribution workflows.

### Changed

- Address focus outline now matches the input's 4 px corners.
- Tab favicons align with titles, and internal favicons use transparent masks.
- Settings and localized product text use NavZip identity while upstream attribution is retained.

### Removed

- Servo/egui application, old installers and unused parallel browser-state/platform crates.
- Bundled new-tab sites, greeting, personal name, city, weather widget and its privileged API.
- Automatic creation of demonstration bookmarks in new profiles.

### Qualification

A macOS Intel development build has passed 16 local native integration checks. C++ test objects were compiled but not executed. Other browser platforms, signed installers, an updater and stable-release qualification remain pending. See [Validation](docs/VALIDATION.md).
