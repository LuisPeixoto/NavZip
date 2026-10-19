# Design

NavZip aims for a calm, minimal interface with useful native behavior. Screenshots document implementation; concepts describe direction. They are not interchangeable evidence.

## Native frame

- Navigation and the address bar occupy the first row, respecting native window controls.
- Horizontal tabs appear below navigation when two or more tabs exist. With one tab, the tab strip disappears and the page regains its space. The same rule applies to the vertical sidebar while retaining the profile's preferred orientation.
- The address field is 30 px high with a 4 px radius. The focus outline uses the same geometry.
- Horizontal tabs are 38 px high. Favicons and titles share a visual center; the active tab has a 2 px mint accent.
- The bookmarks bar appears when enabled. Layout changes preserve native navigation, focus, tab and page state.
- Vertical tabs and real split WebContents remain available through Customize. Split panels have individual controls and follow the focused page for navigation.

## Color and shape

| Token | Dark | Light |
| --- | --- | --- |
| Base | `#101517` | `#f4f7f5` |
| Raised surface | `#202629` | White |
| Accent | `#8cdbb6` | `#16694a` |

Semantic color mixers cover native controls and WebUI, including Settings, selection and focus. Custom themes and high contrast take precedence. Settings cards use 12 px corners with restrained surfaces. Native platform fonts and window controls are retained.

## New tab

The new tab contains a real local clock, functional search, an initially empty list of user shortcuts and Customize. It has no suggested sites, greeting, personal-name field, city or weather service. Earlier bundled shortcut IDs are ignored when reading old preferences; user-created shortcuts are preserved. Existing bookmarks are not deleted.

The central search field has an 874 px maximum width and 78 px height; shortcut tiles are 92 px square at full width. Narrow windows reduce dimensions and wrap controls. The footer contains the NavZip mark and Customize. Light/dark modes follow the native browser theme, including explicit per-profile overrides.

## Accessible interactions

Keep semantic HTML, descriptive accessible names, visible keyboard focus and reduced-motion support. Do not replace working native controls with decorative replicas. Full accessibility and screen-reader qualification remain release gates; markup alone is not proof.

## Visual review

Use the [gallery](SCREENSHOTS.md) for provenance. `npm test` captures only the component with a test bridge and controlled test time. `npm run test:native` captures the real app frame on supported macOS setups. Actual application clocks remain dynamic. Compare spacing, contrast, alignment, one-tab behavior, both orientations and split panels without claiming pixel equivalence across OS fonts and controls.
