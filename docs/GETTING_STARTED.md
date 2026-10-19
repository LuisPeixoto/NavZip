# Getting started

NavZip is an experimental Chromium browser. The repository is public so contributors can inspect, build and improve it; it is not yet a stable browser distribution.

## I want to install it

There is currently no qualified, signed Chromium installer to recommend. Follow the [release page](https://github.com/LuisPeixoto/NavZip/releases) and [release gates](RELEASE.md). Do not substitute an old Servo Alpha package: it uses a different engine and is no longer maintained here.

If you are comfortable with native development and the disk requirements, follow [Building](BUILDING.md). A component development build depends on libraries in its checkout and cannot simply be dragged to another computer.

## I have built NavZip

1. Launch with `cargo xtask run -- --no-first-run`.
2. Use the address bar or central search field to navigate.
3. Choose **Adicionar** (Add) on the new tab to save your own shortcut.
4. Use **Personalizar** (Customize) to manage shortcuts, switch tab orientation or create a split view.
5. Open another tab with **Command+T** on macOS or **Ctrl+T** on Windows/Linux. The tab bar is hidden while only one tab is open.

The current custom interface is in Brazilian Portuguese. English documentation does not imply that the application has complete English localization yet.

| Action | macOS | Windows / Linux |
| --- | --- | --- |
| New tab | Command+T | Ctrl+T |
| Focus address bar | Command+L | Ctrl+L |
| Focus new-tab search | Command+K | Ctrl+K |
| Close current tab | Command+W | Ctrl+W |
| Reload | Command+R | Ctrl+R |

## I want to help

You can improve docs or the WebUI without downloading Chromium. Follow [Contributing](../CONTRIBUTING.md), then run the checks relevant to your changes. Browser-native contributions require the full checkout and native checks.

Keep experimental browsing profiles separate from your primary browser. Do not put credentials, private profiles or personal downloads in issues or pull requests.
