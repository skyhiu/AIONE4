// The Boot Up / Boot Down preview: the same module AIONE 4 runs, framed like the reference page.
// Frames follow the sound's clock (audio.currentTime), as in the app.
import { BOOT_DOWN_MS, BOOT_UP_MS, CLAMP_MS, createWindowB } from '../../../src/lib/boot/windowB';
import upUrl from '../../../assets/sounds/boot/aione_boot_up.wav?url';
import downUrl from '../../../assets/sounds/boot/aione_boot_down.wav?url';

const wb = createWindowB("system-ui, -apple-system, 'Segoe UI', sans-serif");
const stage = document.getElementById('stage') as HTMLDivElement;
stage.appendChild(wb.svg);
const bg = document.createElementNS('http://www.w3.org/2000/svg', 'rect');
for (const [k, v] of Object.entries({ x: -340, y: -230, width: 680, height: 460, fill: '#000' })) bg.setAttribute(k, String(v));
wb.svg.insertBefore(bg, wb.svg.firstChild);
let raf = 0;
let audio: HTMLAudioElement | null = null;
const info = document.getElementById('info') as HTMLParagraphElement;

function run(fn: (t: number) => void, from: number, to: number, speed: number, clock?: () => number) {
  cancelAnimationFrame(raf);
  const start = performance.now();
  let clampLogged = false;
  const step = (now: number) => {
    const t = Math.min(clock ? clock() : from + (now - start) * speed, to);
    fn(t);
    if (clock && !clampLogged && t >= CLAMP_MS && fn === wb.bootUpFrame) {
      clampLogged = true;
      info.textContent = `Clamp drawn at ${t.toFixed(1)} ms of the sound (target ${CLAMP_MS} ms, one frame ≈ 16.7 ms).`;
    }
    if (t < to) raf = requestAnimationFrame(step);
  };
  raf = requestAnimationFrame(step);
}

function play(up: boolean) {
  audio?.pause();
  const fn = up ? wb.bootUpFrame : wb.bootDownFrame;
  const dur = up ? BOOT_UP_MS : BOOT_DOWN_MS;
  if (!(document.getElementById('snd') as HTMLInputElement).checked) return run(fn, 0, dur, 1);
  const a = new Audio(up ? upUrl : downUrl);
  audio = a;
  a.addEventListener('playing', () => run(fn, 0, dur, 1, () => a.currentTime * 1000), { once: true });
  a.play().catch(() => run(fn, 0, dur, 1));
}

(document.getElementById('up') as HTMLButtonElement).onclick = () => play(true);
(document.getElementById('down') as HTMLButtonElement).onclick = () => play(false);
(document.getElementById('slow') as HTMLButtonElement).onclick = () => {
  audio?.pause();
  run(wb.bootUpFrame, 1800, CLAMP_MS + 900, 0.2);
};
wb.bootUpFrame(4650);
// Test hook: draw one moment.
(window as unknown as { __wb: unknown }).__wb = { up: wb.bootUpFrame, down: wb.bootDownFrame };
