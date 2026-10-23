// The status header's icon names (Material Symbols Rounded; A16: each name exists in the pinned
// font, checked by tests/unit/status-icons.test.ts and `check:assets`).
import type { PcStatus } from '../core/types';

export function batteryIcon(b: NonNullable<PcStatus['battery']>): string {
  if (b.charging) return 'battery_android_frame_bolt';
  if (b.percent === null) return 'battery_android_frame_question';
  if (b.percent >= 95) return 'battery_android_frame_full';
  const bars = Math.min(6, Math.round((b.percent / 100) * 6));
  // The font has frames 1–6 but no 0 (alpha.37: under about 8 % the header showed the raw text
  // "battery_android_frame_0"); an empty battery is its alert frame.
  return bars < 1 ? 'battery_android_frame_alert' : `battery_android_frame_${bars}`;
}
