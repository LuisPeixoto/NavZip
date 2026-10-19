# Reference-browser research

Historical static inspection was performed on October 16, 2026 using REA 6.5.0. It informed the engine choice; it is not evidence that proprietary source code was recovered.

| Installed artifact | Local observation | Architecture corroborated by official sources |
| --- | --- | --- |
| Chrome 155.0.8059.80 | Universal Mach-O; Chrome framework, libobjc, AppKit/Metal/Security; V8 and ICU resources | Chromium, Blink and V8 |
| Safari 26.2 | Universal Mach-O; private Safari framework and libSystem dependencies | WebKit and Apple frameworks |
| Brave 155.1.97.57 | Universal Mach-O; Brave framework, Chrome/Brave, V8 and ICU resources | Chromium, Blink/V8 and adblock-rust |

Pseudocode extraction was attempted but failed because the Hopper launcher could not locate the app through Spotlight. Dependency inspection is not complete decompilation, does not identify every implementation language and does not establish runtime behavior. No proprietary code or extracted product assets were incorporated into NavZip.

## Local evidence identifiers

These IDs refer to the local investigation, not public downloadable reports:

- Chrome framework: `ev_6674a9ff2e9e6c5ce1fb04d3cfe3c19fbc77c8b21cdaa4b73630117e7bfa91b2`
- Safari: `ev_aa54455f0f2eda72e5e3568401b63b6d27b361fb8b5b7d9b22448e8918f33ba4`
- Brave framework: `ev_37b0d3625abd5e766461c526191632b4c6c5a1b5470b1ce93028fbd859a1f580`

## Sources

[Blink](https://www.chromium.org/blink/) · [V8](https://v8.dev/) · [WebKit](https://webkit.org/project/) · [Brave Core](https://github.com/brave/brave-core) · [Servo](https://github.com/servo/servo)

The former Servo application has been removed. NavZip now builds the pinned open-source Chromium source directly; see [Architecture](ARCHITECTURE.md).
