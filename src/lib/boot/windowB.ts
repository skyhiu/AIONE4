// The default Boot Up and Boot Down: the AIONE logo "Window B" (B-5, final).
//
// A line-for-line port of `docs/brand/aione_boot_animation_reference.html` (the design chat's
// reference, 2026-10-13): the same SVG structure, constants, curves (spring, eo, io, wob, impact)
// and seeded random shards; `bootUpFrame(t)` and `bootDownFrame(t)` are its `fu(t)` and `fd(t)`,
// pure functions of the time t in ms. The only change: the diamond is #FF46A2 (B-9, one brand
// pink) instead of the reference's #FF3D7F. Geometry from `aione_mark_B_4part.svg`: circle r 100,
// window half-diagonal 65, diamond half-diagonal 48, cut 12, tilted 25° clockwise; the four
// quarters meet on y = 0 with no gap (the hidden seams), drawn with a 0.8 stroke in the same
// colour so no background shows through at rest.
//
// The caller drives the frames from the sound's clock, so the clamp (2550 ms) lands with the hit.

export const OFF_WHITE = '#F5F3EE';
export const BRAND_PINK = '#FF46A2';
export const BOOT_UP_MS = 5000;
export const BOOT_DOWN_MS = 1800;
export const CLAMP_MS = 2550;

const NS = 'http://www.w3.org/2000/svg';

export interface WindowB {
  svg: SVGSVGElement;
  bootUpFrame(t: number): void;
  bootDownFrame(t: number): void;
}

const QUARTERS: [string, string, number, number, number][] = [
  ['TL', 'M -6 -99.82 L -6 -59 L -65 0 L -100 0 A 100 100 0 0 1 -6 -99.82 Z', -1, -1, 1],
  ['TR', 'M 6 -99.82 L 6 -59 L 65 0 L 100 0 A 100 100 0 0 0 6 -99.82 Z', 1, -1, -1],
  ['BR', 'M 100 0 L 65 0 L 6 59 L 6 99.82 A 100 100 0 0 0 100 0 Z', 1, 1, -1],
  ['BL', 'M -100 0 L -65 0 L -6 59 L -6 99.82 A 100 100 0 0 1 -100 0 Z', -1, 1, 1],
];

function el<K extends keyof SVGElementTagNameMap>(tag: K, attrs: Record<string, string | number>): SVGElementTagNameMap[K] {
  const e = document.createElementNS(NS, tag);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, String(v));
  return e;
}

/** Builds the reference's SVG (viewBox −340 −230 680 460) and returns its two frame functions. */
export function createWindowB(fontFamily: string): WindowB {
  // A larger viewBox than the reference's, centred the same: on a full screen nothing (shake,
  // ring, shards) is ever clipped; the mark keeps the reference's proportions to the screen
  // height (the caller sizes the SVG).
  const svg = el('svg', { viewBox: '-340 -230 680 460', preserveAspectRatio: 'xMidYMid meet', 'aria-hidden': 'true' });
  svg.style.overflow = 'visible';
  const all = el('g', { id: 'wb-all' });
  const amb = el('g', { transform: 'translate(0 -20)' });
  const SH = el('g', {});
  const inner = el('g', { transform: 'translate(0 -20)' });
  const RING = el('circle', { r: 100, fill: 'none', stroke: OFF_WHITE, 'stroke-width': 6, opacity: 0 });
  const MK = el('g', { fill: OFF_WHITE, stroke: OFF_WHITE, 'stroke-width': 0.8 });
  const Q: [SVGPathElement, number, number, number][] = QUARTERS.map(([id, d, dx, dy, side]) => {
    const p = el('path', { d, opacity: 0, 'data-part': id });
    MK.appendChild(p);
    return [p, dx, dy, side];
  });
  const CT = el('line', { x1: 0, y1: -59, x2: 0, y2: -59, stroke: '#000', 'stroke-width': 3, opacity: 0 });
  const CB = el('line', { x1: 0, y1: 59, x2: 0, y2: 59, stroke: '#000', 'stroke-width': 3, opacity: 0 });
  const shards = el('g', { stroke: 'none' });
  const D = el('polygon', { opacity: 0, stroke: 'none', fill: BRAND_PINK, points: '0,0', 'data-part': 'diamond' });
  MK.append(CT, CB, shards, D);
  inner.append(RING, MK);
  SH.appendChild(inner);
  const word = el('g', { fill: OFF_WHITE, 'font-size': 22, 'font-weight': 500, 'text-anchor': 'middle' });
  word.style.fontFamily = fontFamily;
  const LT = ['A', 'I', 'O', 'N', 'E'].map((ch, i) => {
    const t = el('text', { x: -50 + i * 25, y: 132, opacity: 0 });
    t.textContent = ch;
    word.appendChild(t);
    return t;
  });
  all.append(amb, SH, word);
  svg.appendChild(all);

  // ---- the reference's script, ported ----
  const cl = (x: number) => Math.max(0, Math.min(1, x));
  const p = (t: number, a: number, b: number) => cl((t - a) / (b - a));
  const eo = (x: number) => 1 - Math.pow(1 - x, 3);
  const io = (x: number) => (x < 0.5 ? 4 * x * x * x : 1 - Math.pow(-2 * x + 2, 3) / 2);
  const spring = (x: number, k: number, w: number) => (x <= 0 ? 0 : 1 - Math.exp(-k * x) * Math.cos(w * x));
  const wob = (x: number, a: number, k = 5, w = 13) => (x <= 0 ? 0 : a * Math.exp(-k * x) * Math.sin(w * x));
  const N = 72,
    STOP = 35,
    PULL = 50,
    C = 52,
    HIT = CLAMP_MS,
    FRZ = 2370,
    DASH = 2430,
    CR0 = HIT + 30,
    CR1 = HIT + 100,
    SP0 = HIT + 100,
    M0 = HIT + 450,
    M1 = HIT + 800,
    TU = BOOT_UP_MS;
  let rs = 11;
  const rnd = () => {
    rs = (rs * 16807) % 2147483647;
    return rs / 2147483647;
  };
  const SHD: { el: SVGPolygonElement; y0: number; vx: number; vy: number; rot: number; d: number }[] = [];
  for (let i = 0; i < 10; i++) {
    const s = 3 + rnd() * 4;
    const e = el('polygon', { points: `0,${-s} ${s * 0.9},${s * 0.6} ${-s * 0.8},${s * 0.7}`, opacity: 0, fill: OFF_WHITE });
    shards.appendChild(e);
    const top = i < 5;
    const side = i % 2 ? 1 : -1;
    SHD.push({ el: e, y0: top ? -100 : 100, vx: side * (40 + rnd() * 90), vy: (top ? -1 : 1) * (60 + rnd() * 80), rot: (rnd() - 0.5) * 900, d: 380 + rnd() * 200 });
  }
  // The reference creates ten ambient dots too (they consume the same random numbers, so the
  // sequence stays identical) but never shows them in Boot Up; kept hidden here as there.
  const AMB: SVGCircleElement[] = [];
  for (let i = 0; i < 10; i++) {
    const e = el('circle', { r: 1.6 + rnd() * 1.6, fill: i % 3 === 0 ? BRAND_PINK : OFF_WHITE, opacity: 0 });
    amb.appendChild(e);
    // a, r0, dr, up, s, d: six more draws per dot, as in the reference.
    for (let k = 0; k < 6; k++) rnd();
    AMB.push(e);
  }
  const FADE0 = 4650;
  const shape = (m: number) => {
    const pts: string[] = [];
    for (let i = 0; i < N; i++) {
      const a = (i / N) * Math.PI * 2,
        c = Math.cos(a),
        s = Math.sin(a);
      const r = 44 + (48 / (Math.abs(c) + Math.abs(s)) - 44) * m;
      pts.push(`${(r * c).toFixed(2)},${(r * s).toFixed(2)}`);
    }
    return pts.join(' ');
  };
  const qset = (e: SVGPathElement, dx: number, dy: number, side: number, r: number, sc: number, rotA: number, sep: number, shear: number, op: number, stretch = 1) => {
    const k = r * 0.7071,
      cx = dx * C,
      cy = dy * C,
      ang = (Math.atan2(dy, dx) * 180) / Math.PI;
    e.setAttribute(
      'transform',
      `rotate(${rotA}) translate(${dx * k - side * sep} ${dy * k + side * shear}) translate(${cx} ${cy}) rotate(${ang}) scale(${Math.max(sc * stretch, 0.001)} ${Math.max(sc / stretch, 0.001)}) rotate(${-ang}) translate(${-cx} ${-cy})`,
    );
    e.setAttribute('opacity', String(op));
  };
  const impact = (t: number, depth: number) => {
    if (t < HIT) {
      const pr = p(t, DASH, HIT);
      return 1 - depth * Math.pow(pr, 3);
    }
    const x = p(t, HIT, HIT + 600);
    return 1 - depth * Math.exp(-7 * x) * Math.cos(16 * x);
  };
  let seed = 1;
  const jit = () => {
    seed = (seed * 16807) % 2147483647;
    return (seed / 2147483647) * 2 - 1;
  };

  function bootUpFrame(t: number) {
    D.setAttribute('points', shape(eo(p(t, HIT + 20, HIT + 200))));
    const gx = p(t, 400, 1000),
      grow = spring(gx, 6, 11),
      st = wob(gx, 0.22, 5, 11);
    let core: number;
    if (t < 1850) core = 1;
    else if (t < FRZ) core = 1 + 0.6 * io(p(t, 1850, FRZ)) + 0.012 * Math.sin(t * 0.11) * p(t, 1850, FRZ);
    else if (t < DASH) core = 1.6;
    else if (t < HIT) {
      const z = p(t, DASH, HIT);
      core = 1.6 - 0.84 * z * z;
    } else core = impact(t, 0.24);
    const u = grow * core;
    D.setAttribute('transform', `scale(${Math.max(u / (1 + st), 0.001)} ${Math.max(u * (1 + st), 0.001)})`);
    D.setAttribute('opacity', t > 400 ? '1' : '0');
    MK.setAttribute('transform', `rotate(${25 * io(p(t, M0, M1))}) scale(${impact(t, 0.05)})`);
    const sx = p(t, HIT, HIT + 360),
      amp = t >= HIT ? 10 * Math.pow(1 - sx, 2) : 0;
    seed = 1 + Math.floor(t / 16);
    SH.setAttribute('transform', `translate(${amp * jit()} ${amp * jit()})`);
    all.setAttribute('opacity', String(1 - io(p(t, FADE0, TU))));
    const rx = p(t, HIT, HIT + 280);
    RING.setAttribute('r', String(100 + 60 * eo(rx)));
    RING.setAttribute('stroke-width', String(10 * (1 - rx) + 1));
    RING.setAttribute('opacity', String(t >= HIT && rx < 1 ? 0.6 * (1 - rx) : 0));
    const ck = eo(p(t, CR0, CR1)),
      showCr = t >= CR0 && t < SP0 + 40;
    CT.setAttribute('y2', String(-59 - 46 * ck));
    CB.setAttribute('y2', String(59 + 46 * ck));
    CT.setAttribute('opacity', showCr ? '1' : '0');
    CB.setAttribute('opacity', showCr ? '1' : '0');
    const spx = p(t, SP0, SP0 + 520);
    const sep = t < SP0 ? -6 : -6 + 6 * (1 - Math.exp(-6 * spx) * Math.cos(11 * spx)) + 10 * Math.exp(-5 * spx) * Math.sin(11 * spx);
    const twist = wob(spx, 5, 6, 12);
    for (const o of SHD) {
      const x = p(t, SP0, SP0 + o.d),
        tt = (x * o.d) / 1000;
      o.el.setAttribute('transform', `translate(${o.vx * tt} ${o.y0 + o.vy * tt + 220 * tt * tt}) rotate(${o.rot * x})`);
      o.el.setAttribute('opacity', String(t >= SP0 && x < 1 ? 1 - x : 0));
    }
    Q.forEach(([e, dx, dy, side], i) => {
      const s = 1000 + i * 120,
        x = p(t, s, s + 600);
      let r: number,
        str = 1;
      const sc = spring(x, 6, 11);
      if (t < 1850) r = STOP + 20 * (1 - spring(x, 6, 11));
      else if (t < FRZ) {
        const y = p(t, 1850, FRZ);
        r = STOP + PULL * io(y) + Math.sin(t * 0.11 + i) * 3 * y * y;
      } else if (t < DASH) r = STOP + PULL + 4 * eo(p(t, FRZ, DASH));
      else if (t < HIT) {
        const z = p(t, DASH, HIT);
        r = (STOP + PULL + 4) * (1 - Math.pow(z, 4));
        str = 1 + 0.22 * Math.sin(Math.PI * Math.min(z * 1.1, 1));
      } else {
        const y = p(t, HIT, HIT + 500);
        r = -9 * Math.exp(-7 * y) * Math.cos(16 * y);
        str = 1 - 0.12 * Math.exp(-8 * y) * Math.cos(14 * y);
      }
      qset(e, dx, dy, side, r, sc, -side * twist, sep, 0, x > 0 ? 1 : 0, str);
    });
    LT.forEach((e, i) => {
      const z = p(t, M1 + 100 + i * 90, M1 + 600 + i * 90);
      e.setAttribute('transform', `translate(0 ${14 * (1 - spring(z, 6, 10))})`);
      e.setAttribute('opacity', String(eo(z)));
    });
  }

  const QS = 850,
    QG = 120,
    QL = 350,
    LAST = QS + 3 * QG;
  function bootDownFrame(t: number) {
    SH.setAttribute('transform', '');
    all.setAttribute('opacity', '1');
    AMB.forEach((e) => e.setAttribute('opacity', '0'));
    RING.setAttribute('opacity', '0');
    CT.setAttribute('opacity', '0');
    CB.setAttribute('opacity', '0');
    SHD.forEach((o) => o.el.setAttribute('opacity', '0'));
    LT.forEach((e, i) => {
      const z = p(t, (4 - i) * 50, (4 - i) * 50 + 300);
      e.setAttribute('transform', `translate(0 ${16 * eo(z)})`);
      e.setAttribute('opacity', String(1 - eo(z)));
    });
    const rot = 25 * (1 - io(p(t, 250, 700))) + 4 * Math.sin(Math.PI * p(t, 200, 380));
    MK.setAttribute('transform', `rotate(${rot}) scale(${1 + 0.04 * Math.sin(Math.PI * p(t, 120, 320))})`);
    D.setAttribute('points', shape(1 - io(p(t, LAST + 10, LAST + 190))));
    const x2 = p(t, LAST + 190, LAST + 290),
      x3 = p(t, LAST + 290, LAST + 510),
      sc = (1 + 0.15 * eo(x2)) * (1 - x3 * x3 * x3),
      st = 0.18 * x3;
    D.setAttribute('transform', `scale(${Math.max(sc / (1 + st), 0.001)} ${Math.max(sc * (1 + st), 0.001)})`);
    D.setAttribute('opacity', t < LAST + 510 ? '1' : '0');
    Q.forEach(([e, dx, dy, side], i) => {
      const j = 3 - i,
        x = p(t, QS + j * QG, QS + j * QG + QL);
      const s = x < 0.3 ? 1 + 0.12 * eo(x / 0.3) : 1.12 * (1 - Math.pow((x - 0.3) / 0.7, 3));
      qset(e, dx, dy, side, 20 * eo(x), s, 0, 0, 0, x < 1 ? 1 : 0);
    });
  }

  return { svg, bootUpFrame, bootDownFrame };
}
