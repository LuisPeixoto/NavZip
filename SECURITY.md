# Security policy

## Supported stage

NavZip is an **experimental developer preview**, not a security-qualified stable browser. There is no supported production release or guaranteed security-response timeline yet. The pinned Chromium revision is a reproducibility choice, not a claim of current security support.

## Report a vulnerability privately

Use [GitHub private vulnerability reporting](https://github.com/LuisPeixoto/NavZip/security/advisories/new). Private reporting is enabled for this repository. Do not open a public issue with exploit details, credentials, personal profiles or other sensitive data.

Include the affected revision/platform, impact, minimal reproduction and any mitigations you found. Use a disposable profile and redact secrets. The maintainer will assess reports as capacity allows; no response-time promise or bounty is offered.

## Security boundaries

- Chromium's sandbox, site isolation, TLS and certificate checks must stay enabled.
- The custom new-tab bridge is typed Mojo, available only to trusted internal WebUI. External pages must not receive privileged APIs.
- Shortcut writes validate input and reject off-the-record persistence.
- Private browsing uses Chromium's off-the-record profile. Local cookie/marker checks passed, but complete crash-path/privacy and platform sandbox audits remain pending. Private browsing is not network anonymity.
- The optional legacy importer is explicit, reads a consistent backup and does not transfer cookies, cache or password-vault secrets.
- The experimental Wasm worker has no WASI/network/filesystem/credential bridge and applies memory, fuel, message and time limits. Host-side authorization is still mandatory; interpreter limits are not a platform sandbox.
- Weather/location functionality and its native API were removed. Chromium's own network services still require review; no blanket “zero network traffic” guarantee is made.

## Development and CI

Do not test exploit payloads against other users or services. Do not upload personal profiles or secrets to CI. Source workflows run on hosted runners with minimal token permissions. Full native builds are manual, restricted to `main` and intended for trusted, dedicated runners; do not execute untrusted pull-request code on those runners.

GitHub secret scanning and dependency tooling supplement review; they do not establish browser security. See [release gates](docs/RELEASE.md) for remaining qualification work.
