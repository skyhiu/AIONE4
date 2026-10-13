// The header clock (reference A4): "12:24" with a smaller "PM" in 12-hour mode. Until the
// setting exists (T3) it follows Windows' own clock format (the web view's locale).

export interface ClockText {
  time: string;
  /** "AM" / "PM" in 12-hour mode, else null. */
  ampm: string | null;
}

export function formatClock(d: Date, hour12: boolean): ClockText {
  const m = String(d.getMinutes()).padStart(2, '0');
  if (!hour12) return { time: `${String(d.getHours()).padStart(2, '0')}:${m}`, ampm: null };
  const h = d.getHours() % 12 || 12;
  return { time: `${h}:${m}`, ampm: d.getHours() < 12 ? 'AM' : 'PM' };
}

export function systemUses12h(): boolean {
  const cycle = new Intl.DateTimeFormat(undefined, { hour: 'numeric' }).resolvedOptions().hourCycle;
  return cycle === 'h11' || cycle === 'h12';
}

/** Milliseconds until the next minute starts (the clock's single timer). */
export function msToNextMinute(d: Date): number {
  return 60_000 - (d.getSeconds() * 1000 + d.getMilliseconds());
}
