# Extensions

## Chromium extensions

NavZip retains Chromium's native extension implementation. A local Manifest V3 content script has passed the native integration script. Chrome Web Store installation, updates, permission flows and compatibility across extensions still require validation. No universal compatibility or store availability is promised.

## Experimental NavZip packages

The three examples under `examples/extensions` and the Rust validator/worker are preserved development components. The `.navzip-extension` installation UI, consent flow and action connection to Chromium have **not** been integrated.

Future integration must require explicit consent, start installed packages disabled and keep them disabled by default in private browsing. The authorized manifest must remain identical to the consented manifest when actions are applied.

### Package format

A `.navzip-extension` is ZIP with `manifest.json` at its root. `manifest_version` is `1`; IDs accept lowercase letters, numbers and hyphens. Limits are 16 MiB compressed/expanded, 128 members and a 64 KiB manifest. Absolute paths, traversal, symlinks and reserved names are rejected.

Permissions include `commands`, `tabs.read`, `tabs.create`, `tabs.close`, `theme`, `storage` and `content.scripts`. `tabs.read` is reserved: the initial ABI emits actions and does not receive a tab list. Origin patterns check scheme, port and domain boundaries, including patterns such as `https://example.org/*` and `https://*.example.org/*`.

### Wasm ABI

| Contract | Value |
| --- | --- |
| Exports | `memory`, `run: () -> i32`; zero means success |
| Allowed import | `navzip.emit: (i32 pointer, i32 length) -> i32` |
| Message | UTF-8 JSON, at most 64 KiB |
| Actions | At most 32 per execution |
| Memory | At most 16 MiB |
| Fuel | 1,000,000 units |
| Execution deadline | 3 seconds after `NAVZIP-WORKER/1 READY` |
| Initialization deadline | Separately limited to 30 seconds |
| Storage | Per-extension writes up to 64 KiB; reads are not implemented in the initial ABI |

Examples of emitted actions:

```json
{"action":"create_tab","url":"https://example.org/"}
```

```json
{"action":"close_active_tab"}
```

```json
{"action":"set_theme","accent":[140,219,182],"dark":true}
```

```json
{"action":"store","key":"example","value":"value"}
```

There is no WASI or direct network, filesystem or credential access. The host validates actions again and must authorize them independently. A process boundary and interpreter limits do not establish a platform sandbox.

Userscripts and styles will require per-origin consent, run after page loading and receive no privileged OS bridge. They do not isolate a script from the page itself.

### Build the Rust example

```sh
rustup target add wasm32-unknown-unknown
cargo build --manifest-path examples/extensions/open-rust/rust/Cargo.toml --target wasm32-unknown-unknown --release
```

The library is `no_std`. The accompanying WAT fixture exercises the same ABI in tests. Building an example does not install it into the browser.
