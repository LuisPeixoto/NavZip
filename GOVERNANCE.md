# Governance

NavZip is maintained by [Luis Peixoto](https://github.com/LuisPeixoto). It is an early-stage open-source project; no foundation, corporate backing, support SLA or paid security program is implied.

## Decisions

Routine fixes can be proposed directly as focused pull requests. Changes to architecture, Chromium pins, security boundaries, dependencies, extension permissions or release policy should begin with an issue describing the problem, alternatives and evidence.

The maintainer has final merge and release responsibility. Decisions should be explained in the issue or pull request so future contributors can understand the tradeoff. Review applies to maintainers and contributors; automation results do not replace judgment.

## Branches and history

- `main` is the integration branch. Experimental code on `main` is not a stable release.
- Work uses focused branches and pull requests, with English commit subjects.
- Keep meaningful commit boundaries; use merge commits for coherent multi-commit work or squash small iterations as appropriate.
- Required source checks guard `main`; force-pushing and deleting the default branch are restricted.
- Native runner workflows are manual and must not execute untrusted fork code.

## Release authority

Only explicitly qualified artifacts should be published as stable releases. No source CI badge implies signed binaries, installer qualification, security equivalence with upstream browsers or support for millions of users. The [release gates](docs/RELEASE.md) define the standard.

## Growing the project

Additional reviewers and maintainers can be invited after sustained, constructive contributions. Review rights, release responsibilities and security contacts should be documented when they change. Avoid introducing governance roles or promises that the project cannot yet sustain.
