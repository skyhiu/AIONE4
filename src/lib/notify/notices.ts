// T2.3 notifications: AIONE's own notices (a game ended, a controller connected / disconnected,
// a battery low), one design, one at a time. Pure logic (no timers, no DOM): the app owns the one
// timer (the shown notice's end) and calls `step` when something changes. Tested in
// tests/unit/notices.test.ts.
//
// Rules (Master Prompt T2.3; assumptions stated in the alpha.36 handoff):
// - one notice shows at a time, for SHOW_MS; the others wait in order (at most MAX_WAITING);
// - a newer notice about the same subject (its `key`, e.g. controller 1's connection) replaces a
//   waiting one, so a quick disconnect / reconnect shows only how things stand now;
// - a notice never covers a dialog: while one is open (or AIONE is busy: Boot Up / Down, a
//   launch, a return, a close, or a game in front), notices wait;
// - a controller or battery notice that waited longer than STALE_MS is dropped (it's no longer
//   news); a game's end is shown when AIONE is free again, however long that took;
// - notices are never focusable (passive, like Switch's), so B-29's edge bump doesn't apply.

/** A16: each notice's icon is a Material Symbols Rounded name (checked by `check:assets`); the
 *  controller and battery ones are the header's own symbols (StatusHeader), so they read the same. */
export const NOTICE_ICONS = {
  game: 'videogame_asset',
  pad: 'sports_esports',
  battery: 'battery_android_frame_alert',
} as const;
export type NoticeIcon = (typeof NOTICE_ICONS)[keyof typeof NOTICE_ICONS];

export interface Notice {
  /** The subject: a newer notice with the same key replaces a waiting one. */
  key: string;
  icon: NoticeIcon;
  /** Drawn in the dimmed icon colour, as the header shows a missing controller. */
  dim?: boolean;
  title: string;
  detail?: string;
  /** When it happened (ms, the app's clock). */
  at: number;
  /** Dropped if it has waited longer than STALE_MS (controller and battery notices). */
  expires: boolean;
}

export interface NoticeQueue {
  showing: Notice | null;
  /** When the shown notice appeared (ms). */
  shownAt: number;
  waiting: Notice[];
}

export const SHOW_MS = 4000;
export const STALE_MS = 30000;
export const MAX_WAITING = 3;

export const emptyQueue = (): NoticeQueue => ({ showing: null, shownAt: 0, waiting: [] });

/** Adds a notice; returns the queue and what was dropped to make room (for the log). */
export function push(q: NoticeQueue, n: Notice): { q: NoticeQueue; dropped: Notice[] } {
  const waiting = q.waiting.filter((w) => w.key !== n.key);
  const replaced = q.waiting.filter((w) => w.key === n.key);
  waiting.push(n);
  const dropped = [...replaced];
  while (waiting.length > MAX_WAITING) dropped.push(waiting.shift() as Notice);
  return { q: { ...q, waiting }, dropped };
}

/**
 * Advances the queue at `now`: ends the shown notice after SHOW_MS; shows the next waiting one when
 * nothing is blocking; drops stale ones. Returns what happened, for the log.
 */
export function step(
  q: NoticeQueue,
  now: number,
  blocked: boolean,
): { q: NoticeQueue; ended: Notice | null; shown: Notice | null; stale: Notice[] } {
  let showing = q.showing;
  let ended: Notice | null = null;
  if (showing && now - q.shownAt >= SHOW_MS) {
    ended = showing;
    showing = null;
  }
  const stale = q.waiting.filter((w) => w.expires && now - w.at > STALE_MS);
  let waiting = q.waiting.filter((w) => !stale.includes(w));
  let shown: Notice | null = null;
  let shownAt = q.shownAt;
  if (!showing && !blocked && waiting.length > 0) {
    shown = waiting[0] as Notice;
    waiting = waiting.slice(1);
    showing = shown;
    shownAt = now;
  }
  return { q: { showing, shownAt, waiting }, ended, shown, stale };
}

/** When the app's one timer should next call `step` (ms from now), or null when nothing is due. */
export function nextDue(q: NoticeQueue, now: number): number | null {
  if (q.showing) return Math.max(0, q.shownAt + SHOW_MS - now);
  return null;
}

/** A game's end: "<game> ended", with the time played when there was some. */
export function gameEnded(name: string, minutes: number, at: number): Notice {
  const played =
    minutes >= 1 ? `Played ${Math.round(minutes)} min` : minutes > 0 ? 'Played less than a minute' : undefined;
  return { key: `game:${name}`, icon: NOTICE_ICONS.game, title: `${name} ended`, ...(played ? { detail: played } : {}), at, expires: false };
}

/** A controller connected or disconnected (slot 1–4). */
export function padConnection(slot: number, connected: boolean, at: number): Notice {
  return {
    key: `pad:${slot}`,
    icon: NOTICE_ICONS.pad,
    ...(connected ? {} : { dim: true }),
    title: connected ? 'Controller connected' : 'Controller disconnected',
    detail: `Controller ${slot}`,
    at,
    expires: true,
  };
}

/**
 * A controller's battery (the core's words: "empty", "low", "medium", "full", "wired", "unknown"):
 * a notice when it drops to low or empty, once per level per connection. `last` is the level last
 * noticed for this controller (null after its connection).
 */
export function padBattery(
  slot: number,
  word: string,
  last: string | null,
  at: number,
): { notice: Notice | null; last: string | null } {
  if (word !== 'low' && word !== 'empty') return { notice: null, last: word === 'unknown' ? last : null };
  if (last === word || (last === 'empty' && word === 'low')) return { notice: null, last };
  return {
    notice: {
      key: `padbattery:${slot}`,
      icon: NOTICE_ICONS.battery,
      title: word === 'empty' ? 'Controller battery empty' : 'Controller battery low',
      detail: `Controller ${slot}: charge it or change its batteries`,
      at,
      expires: true,
    },
    last: word,
  };
}

/** The PC's battery levels that get a notice (laptops and tablets). */
export const PC_BATTERY_STEPS = [20, 10] as const;

/**
 * The PC's battery (from the header's status): a notice when it falls to 20 % and to 10 % while not
 * charging; charging (or rising above 20 %) starts over. `last` is the step last noticed (null: none).
 */
export function pcBattery(
  battery: { percent: number | null; charging: boolean } | null,
  last: number | null,
  at: number,
): { notice: Notice | null; last: number | null } {
  if (!battery || battery.percent === null) return { notice: null, last };
  if (battery.charging || battery.percent > (PC_BATTERY_STEPS[0] as number)) return { notice: null, last: null };
  const reached = PC_BATTERY_STEPS.filter((s) => battery.percent !== null && battery.percent <= s).at(-1) ?? null;
  if (reached === null || (last !== null && last <= reached)) return { notice: null, last };
  return {
    notice: {
      key: 'pcbattery',
      icon: NOTICE_ICONS.battery,
      title: 'Battery low',
      detail: `${battery.percent} % left: connect the charger`,
      at,
      expires: true,
    },
    last: reached,
  };
}

/** One log line for a notice. */
export function describe(n: Notice): string {
  return `"${n.title}"${n.detail ? ` (${n.detail})` : ''}`;
}
