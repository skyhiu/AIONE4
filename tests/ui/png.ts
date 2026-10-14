// A small PNG reader for the UI tests (Playwright screenshots: 8-bit RGB or RGBA,
// non-interlaced). Used to measure ink and sample colours (B-8 arrow size, D-011 ring).
import { inflateSync } from 'node:zlib';

export interface Pixels {
  width: number;
  height: number;
  /** RGBA, 4 bytes per pixel. */
  data: Uint8Array;
}

export function decodePng(buf: Buffer): Pixels {
  let pos = 8;
  let width = 0;
  let height = 0;
  let colorType = 6;
  const idat: Buffer[] = [];
  while (pos < buf.length) {
    const len = buf.readUInt32BE(pos);
    const type = buf.toString('latin1', pos + 4, pos + 8);
    const body = buf.subarray(pos + 8, pos + 8 + len);
    if (type === 'IHDR') {
      width = body.readUInt32BE(0);
      height = body.readUInt32BE(4);
      if (body[8] !== 8 || body[12] !== 0) throw new Error('only 8-bit non-interlaced PNGs');
      colorType = body[9] ?? 6;
    } else if (type === 'IDAT') idat.push(body);
    else if (type === 'IEND') break;
    pos += 12 + len;
  }
  const bpp = colorType === 6 ? 4 : colorType === 2 ? 3 : 0;
  if (!bpp) throw new Error(`colour type ${colorType} not supported`);
  const raw = inflateSync(Buffer.concat(idat));
  const stride = width * bpp;
  const out = new Uint8Array(width * height * 4);
  const prev = new Uint8Array(stride);
  const cur = new Uint8Array(stride);
  for (let y = 0; y < height; y++) {
    const f = raw[y * (stride + 1)] ?? 0;
    for (let i = 0; i < stride; i++) {
      const x = raw[y * (stride + 1) + 1 + i] ?? 0;
      const a = i >= bpp ? (cur[i - bpp] ?? 0) : 0;
      const b = prev[i] ?? 0;
      const c = i >= bpp ? (prev[i - bpp] ?? 0) : 0;
      let v = x;
      if (f === 1) v = x + a;
      else if (f === 2) v = x + b;
      else if (f === 3) v = x + ((a + b) >> 1);
      else if (f === 4) {
        const p = a + b - c;
        const pa = Math.abs(p - a);
        const pb = Math.abs(p - b);
        const pc = Math.abs(p - c);
        v = x + (pa <= pb && pa <= pc ? a : pb <= pc ? b : c);
      }
      cur[i] = v & 255;
    }
    for (let px = 0; px < width; px++) {
      const o = (y * width + px) * 4;
      out[o] = cur[px * bpp] ?? 0;
      out[o + 1] = cur[px * bpp + 1] ?? 0;
      out[o + 2] = cur[px * bpp + 2] ?? 0;
      out[o + 3] = bpp === 4 ? (cur[px * bpp + 3] ?? 255) : 255;
    }
    prev.set(cur);
  }
  return { width, height, data: out };
}

export function pixel(p: Pixels, x: number, y: number): [number, number, number] {
  const o = (Math.round(y) * p.width + Math.round(x)) * 4;
  return [p.data[o] ?? 0, p.data[o + 1] ?? 0, p.data[o + 2] ?? 0];
}

/** The bounding box of pixels for which `isInk` is true (null if none). */
export function inkBox(p: Pixels, isInk: (rgb: [number, number, number]) => boolean) {
  let top = Infinity;
  let bottom = -1;
  let left = Infinity;
  let right = -1;
  for (let y = 0; y < p.height; y++)
    for (let x = 0; x < p.width; x++)
      if (isInk(pixel(p, x, y))) {
        top = Math.min(top, y);
        bottom = Math.max(bottom, y);
        left = Math.min(left, x);
        right = Math.max(right, x);
      }
  return bottom < 0 ? null : { top, bottom, left, right, height: bottom - top + 1, width: right - left + 1 };
}

export function contrast(a: [number, number, number], b: [number, number, number]): number {
  const lum = (rgb: [number, number, number]) => {
    const [r, g, bl] = rgb.map((v) => {
      const s = v / 255;
      return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
    });
    return 0.2126 * (r ?? 0) + 0.7152 * (g ?? 0) + 0.0722 * (bl ?? 0);
  };
  const [x, y] = [lum(a), lum(b)];
  return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05);
}
