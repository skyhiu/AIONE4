// B-29 (rev. az): the edge bump, built ONCE in the shared focus system. Every screen whose focus
// can't move further calls App.edge(dir, kind, what, zone): D-019's rules (once per hold, per
// direction) and the edge-bump sound live there; the motion lives here. Home keeps its own B-14
// zones (the row with its wrap, the drawer); every other zone is played here, on its focus ring
// (`.ring`) or, for a slider, on its knob. The item itself stays still; transform only (L-16).

export type BumpDir = 'left' | 'right' | 'up' | 'down';

/** Where each zone's focus is drawn. A new screen adds its zone here (and a `.ring`). */
export const BUMP_TARGETS: Record<string, string> = {
  qm: '[data-testid="quick-menu"] .item.focused .ring',
  po: '[data-testid="power-options"] .item.focused .ring',
  dialog: '[role="alertdialog"] button.focused .ring', // every dialog (rev. ba: the copy question too)
  'slider-brightness': '[data-testid="qm-brightness-value"] .knob',
  'slider-volume': '[data-testid="qm-volume-value"] .knob',
};

/** B-14's spring (the row's ends, measured): about 14 px toward the press, an overshoot of about
 *  6 px, a small wobble, settled in about 150 ms. */
export function playBump(zone: string, dir: BumpDir, n: number): boolean {
  const sel = BUMP_TARGETS[zone];
  const el = sel ? document.querySelector<HTMLElement>(sel) : null;
  if (!el) return false;
  const px = window.innerHeight / 1080; // 1080p / 100 % = 1
  const horizontal = dir === 'left' || dir === 'right';
  const sign = dir === 'left' || dir === 'up' ? -1 : 1;
  const t = (v: number) => (horizontal ? `translateX(${sign * v * px}px)` : `translateY(${sign * v * px}px)`);
  const [out, back, wob, dur] = [14, 6, 2, 150];
  el.dataset['bump'] = String(n);
  el.dataset['bumpDir'] = dir;
  el.animate(
    [
      { transform: t(0) },
      { transform: t(out), offset: 16 / dur },
      { transform: t(-back), offset: 66 / dur, easing: 'ease-in-out' },
      { transform: t(wob), offset: 110 / dur, easing: 'ease-in-out' },
      { transform: t(-wob / 2), offset: 135 / dur, easing: 'ease-in-out' },
      { transform: t(0) },
    ],
    { duration: dur, composite: 'add' },
  );
  return true;
}
