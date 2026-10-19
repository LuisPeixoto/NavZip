import {NewTabPageProxy} from '../new_tab_page_proxy.js';
import type {Bridge} from './types.js';

const proxy = NewTabPageProxy.getInstance();
const handler = proxy.handler;
// Use the browser's actual theme, including an explicit profile override.
proxy.callbackRouter.setTheme.addListener(theme => {
  document.documentElement.dataset['colorSchemeSource'] = 'native';
  document.documentElement.dataset['colorScheme'] = theme.isDark ? 'dark' : 'light';
});
export const bridge: Bridge = {
  async load() { return (await handler.getNavZipPreferences()).preferences; },
  async save(preferences) { return (await handler.setNavZipPreferences(preferences)).accepted; },
  async vertical(enabled) { return (await handler.setNavZipVerticalTabs(enabled)).accepted; },
  async split() { return (await handler.newNavZipSplit()).accepted; },
};
