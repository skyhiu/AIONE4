// The performance log (Master Prompt A7): a `perf` line about every 10 s and on every screen
// change, with the screen on top, frame time avg / p95 over 5 s (requestAnimationFrame), long
// tasks (PerformanceObserver), running animations and timers with their owners, the idle flag
// (no input for 3 s), and what counted as input since the last line.
//
// Cost: frames are sampled with requestAnimationFrame for 5 s before each line (half the time);
// nothing else runs. Owner of both timers: perf.

export const PERF_EVERY_MS = 10_000;
export const SAMPLE_MS = 5_000;
export const IDLE_AFTER_MS = 3_000;

export function frameStats(deltas: number[]): { avg: number; p95: number; n: number } {
  if (deltas.length === 0) return { avg: 0, p95: 0, n: 0 };
  const sorted = [...deltas].sort((a, b) => a - b);
  const avg = deltas.reduce((a, b) => a + b, 0) / deltas.length;
  const p95 = sorted[Math.min(sorted.length - 1, Math.ceil(sorted.length * 0.95) - 1)] ?? 0;
  return { avg, p95, n: deltas.length };
}

type Source = 'controller' | 'keyboard' | 'mouse';

export class PerfLog {
  private screen = 'start';
  private inputs: Record<Source, number> = { controller: 0, keyboard: 0, mouse: 0 };
  private lastInput = performance.now();
  private longTasks = 0;
  private longTaskMs = 0;
  private timer: ReturnType<typeof setTimeout> | null = null;
  private sampler = 0;
  private deltas: number[] = [];
  private observer: PerformanceObserver | null = null;
  private timers: () => string[] = () => [];

  constructor(private readonly log: (line: string) => void) {}

  start(timers: () => string[]): void {
    this.timers = timers;
    try {
      this.observer = new PerformanceObserver((list) => {
        for (const e of list.getEntries()) {
          this.longTasks += 1;
          this.longTaskMs += e.duration;
        }
      });
      this.observer.observe({ type: 'longtask', buffered: false });
    } catch {
      this.observer = null;
    }
    this.cycle();
  }

  stop(): void {
    if (this.timer) clearTimeout(this.timer);
    cancelAnimationFrame(this.sampler);
    this.observer?.disconnect();
  }

  input(source: Source): void {
    this.inputs[source] += 1;
    this.lastInput = performance.now();
  }

  setScreen(screen: string): void {
    if (screen === this.screen) return;
    this.screen = screen;
    this.write(`screen change to ${screen}`);
  }

  private cycle(): void {
    // Wait 5 s, then sample frames for 5 s, then write the line.
    this.timer = setTimeout(() => {
      this.deltas = [];
      let last = performance.now();
      const until = last + SAMPLE_MS;
      const tick = (t: number) => {
        this.deltas.push(t - last);
        last = t;
        if (t < until) this.sampler = requestAnimationFrame(tick);
        else {
          this.write('every 10 s');
          this.cycle();
        }
      };
      this.sampler = requestAnimationFrame(tick);
    }, PERF_EVERY_MS - SAMPLE_MS);
  }

  private write(why: string): void {
    const f = frameStats(this.deltas.slice(1));
    const idleFor = performance.now() - this.lastInput;
    const anims = document
      .getAnimations()
      .map((a) => {
        const el = (a.effect as KeyframeEffect | null)?.target as HTMLElement | null;
        const owner = el?.closest<HTMLElement>('[data-owner]')?.dataset['owner'] ?? 'unknown';
        const name = a instanceof CSSAnimation ? a.animationName : a instanceof CSSTransition ? `transition ${a.transitionProperty}` : a.id || 'script';
        return `${name} (${owner})`;
      });
    const uniq = [...new Set(anims)];
    this.log(
      [
        `perf (${why}): screen ${this.screen}`,
        f.n > 0
          ? `frames ${(f.n * f.avg / 1000).toFixed(1)} s: avg ${f.avg.toFixed(1)} ms, p95 ${f.p95.toFixed(1)} ms (${f.n} frames)`
          : 'frames: not sampled yet',
        `long tasks ${this.longTasks}${this.longTasks ? ` (${Math.round(this.longTaskMs)} ms)` : ''}`,
        `animations ${uniq.length ? uniq.join(', ') : 'none'}`,
        `timers ${['perf (10 s)', ...this.timers()].join(', ')}`,
        `idle ${idleFor >= IDLE_AFTER_MS ? 'yes' : 'no'} (last input ${(idleFor / 1000).toFixed(1)} s ago)`,
        `input since last line: controller ${this.inputs.controller}, keyboard ${this.inputs.keyboard}, mouse ${this.inputs.mouse}`,
      ].join('; '),
    );
    this.inputs = { controller: 0, keyboard: 0, mouse: 0 };
    this.longTasks = 0;
    this.longTaskMs = 0;
  }
}
