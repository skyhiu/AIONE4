// Keys the WebView would otherwise use for browser features (reload, find, print, …).
// AIONE is an app, not a browser page, so these do nothing. Alt+F4 is the OS and still closes.
interface KeyLike {
  key: string;
  ctrlKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
}

const BLOCKED_PLAIN = new Set(['F1', 'F3', 'F5', 'F7', 'F12', 'BrowserBack', 'BrowserForward', 'BrowserRefresh']);
const BLOCKED_CTRL = new Set(['r', 'f', 'g', 'p', 's', 'u', 'o', 'n', 't', 'w', 'j', 'h', '+', '-', '=', '0']);

export function isBlockedBrowserKey(e: KeyLike): boolean {
  if (BLOCKED_PLAIN.has(e.key)) return true;
  if (e.altKey && (e.key === 'ArrowLeft' || e.key === 'ArrowRight')) return true; // history navigation
  return e.ctrlKey && BLOCKED_CTRL.has(e.key.toLowerCase());
}

/** T0 only: Ctrl+Shift+R restarts AIONE 4 (the Power menu comes in T2). */
export function isRestartShortcut(e: KeyLike): boolean {
  return e.ctrlKey && e.shiftKey && !e.altKey && e.key.toLowerCase() === 'r';
}
