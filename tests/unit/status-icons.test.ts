import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { batteryIcon } from '../../src/lib/frame/statusIcons';
import { NOTICE_ICONS } from '../../src/lib/notify/notices';

// A16: every icon name the UI computes exists in the pinned Material Symbols Rounded font
// (a missing name shows as its raw text, e.g. "battery_android_frame_0").
const MATERIAL = new Set(
  [...readFileSync('node_modules/material-symbols/index.d.ts', 'utf8').matchAll(/^\s*"([a-z0-9_]+)",?$/gm)].map((m) => m[1]),
);

describe('A16: computed icon names exist in Material Symbols Rounded', () => {
  it('the header battery, at every percentage, charging and unknown', () => {
    const missing: string[] = [];
    for (let percent = 0; percent <= 100; percent++) {
      const name = batteryIcon({ percent, charging: false });
      if (!MATERIAL.has(name)) missing.push(`${percent} % → ${name}`);
    }
    for (const b of [{ percent: 40, charging: true }, { percent: null, charging: false }]) {
      const name = batteryIcon(b);
      if (!MATERIAL.has(name)) missing.push(name);
    }
    expect(missing).toEqual([]);
  });
  it('the notifications', () => {
    for (const name of Object.values(NOTICE_ICONS)) expect(MATERIAL.has(name), name).toBe(true);
  });
});
