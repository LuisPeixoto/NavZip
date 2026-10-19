# Contributing to NavZip

Thanks for helping make a browser that is clear to use and possible to inspect. Small, well-evidenced contributions are welcome. Read the [community guidelines](CODE_OF_CONDUCT.md) and [security policy](SECURITY.md) first.

## Choose a starting point

- Improve or reproduce a build instruction.
- Report a focused bug with the platform, revision and safe reproduction steps.
- Review keyboard use, contrast, focus and screen-reader behavior.
- Improve the new-tab component without downloading Chromium.
- Add meaningful regression coverage around migration, plugins or native integration.

Check existing issues before starting a large change. Discuss architecture, engine upgrades, dependencies and new user-facing features before investing in a broad implementation. A large rewrite is harder to review than a sequence of small changes.

## Development setup

Follow [Building](docs/BUILDING.md). For component/tooling work, start with Rust, Node/npm and the source checks; Chromium-native changes require the full pinned checkout.

```sh
git clone https://github.com/LuisPeixoto/NavZip.git
cd NavZip
git switch -c feat/describe-your-change
cargo test --workspace --locked
npm ci --ignore-scripts
npx playwright install --with-deps chromium
npm test
```

External contributors should fork the repository and open a pull request from their fork.

## Project conventions

- Use **English** for documentation, code comments, commit subjects, issues and pull requests. User-visible strings may be localized; do not translate unrelated UI as part of a docs change.
- Use focused branches from `main`, such as `feat/shortcut-editing`, `fix/tab-focus` or `docs/linux-build`. Automated work may use `codex/` branches.
- Use Conventional Commit-style subjects: `feat:`, `fix:`, `docs:`, `test:`, `ci:`, `refactor:` or `chore:`; a useful scope is welcome.
- Keep independent changes in separate commits. Describe final behavior, not the chat or abandoned approaches.
- Rust is preferred for supporting tools where practical; browser-native integration follows Chromium's C++/Views and Mojo conventions.
- Respect existing platform behavior, accessibility, profile boundaries and unrelated local changes.
- Keep upstream patches and owned overlays small, documented and pinned. [Patch workflow](docs/CHROMIUM.md).

## Required checks

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
npm test
npm run check:repo
```

For native C++/Mojo or browser integration changes:

```sh
cargo xtask build-integration
cargo xtask build
cargo xtask check
npm run test:native
```

Report what actually ran. Source CI, component tests, compiled C++ test objects and native integration tests prove different things. Do not claim C++ tests executed if only their objects were compiled. Do not claim Windows/Linux browser support from hosted Rust CI.

## Pull request checklist

- Explain the concrete problem and resulting user/developer behavior.
- Link a related issue when applicable.
- Include relevant tests and results, plus platform limitations.
- For visuals, include non-sensitive before/after captures and identify native versus component captures.
- Update docs when commands, interfaces or user-visible behavior change.
- Preserve license and attribution notices.
- Do not commit user profiles, credentials, `.env` files, checkout trees, caches, downloads, build artifacts or diagnostic logs with private data.

Do not disable TLS, certificate checks, sandboxing or origin policies to make a test pass. Private sessions must not be persisted. Privileged APIs must remain restricted to the trusted internal WebUI.

## Review and merge

The maintainer reviews scope, behavior, evidence and release impact. Required source checks must pass before merge. Use a merge commit when preserving a meaningful multi-commit implementation; squash small iterative fixes when appropriate. Avoid force-pushing shared branches. See [Governance](GOVERNANCE.md).

Submitting a contribution means you have the right to contribute it under the applicable project/upstream licenses. No CLA is currently required. This does not transfer third-party trademarks or remove upstream license obligations.
