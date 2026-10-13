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

/** D-008: a frame over this is "janky" (a visible freeze); frames over SLOW_FRAME_MS are counted. */
export const JANK_FRAME_MS = 50;
export const SLOW_FRAME_MS = 33;

export interface FrameStats {
  avg: number;
  p95: number;
  n: number;
  /** D-008: the longest frame, frames over 33 ms, and whether any frame was over 50 ms. */
  longest: number;
  over33: number;
  janky: boolean;
}

export function frameStats(deltas: number[]): FrameStats {
  if (deltas.length === 0) return { avg: 0, p95: 0, n: 0, longest: 0, over33: 0, janky: false };
  const sorted = [...deltas].sort((a, b) => a - b);
  const avg = deltas.reduce((a, b) => a + b, 0) / deltas.length;
  const p95 = sorted[Math.min(sorted.length - 1, Math.ceil(sorted.length * 0.95) - 1)] ?? 0;
  const longest = sorted[sorted.length - 1] ?? 0;
  return {
    avg,
    p95,
    n: deltas.length,
    longest,
    over33: deltas.filter((d) => d > SLOW_FRAME_MS).length,
    janky: longest > JANK_FRAME_MS,
  };
}

/** The frames part of a perf line (pure; tested). */
export function describeFrames(f: FrameStats, maxInputDelay: number | null): string {
  if (f.n === 0) return 'frames: not sampled yet';
  return (
    `frames ${((f.n * f.avg) / 1000).toFixed(1)} s: avg ${f.avg.toFixed(1)} ms, p95 ${f.p95.toFixed(1)} ms, ` +
    `longest ${f.longest.toFixed(1)} ms, over 33 ms: ${f.over33} (${f.n} frames)${f.janky ? ', JANKY' : ''}; ` +
    `longest input delay (core → UI) ${maxInputDelay === null ? 'none' : `${Math.round(maxInputDelay)} ms`}`
  );
}

/** A picture the page finished loading or decoding (D-007 attribution). */
export interface PictureEvent {
  at: number;
  what: string;
  width: number;
  height: number;
}

/** D-007: one line for a long task, naming what it overlapped (pure; tested). Pictures count
 *  when they finished within 1 s before the task ended. */
export function describeLongTask(
  start: number,
  duration: number,
  lastAction: string,
  pictures: PictureEvent[],
): string {
  const end = start + duration;
  const near = pictures.filter((p) => p.at >= start - 1000 && p.at <= end + 50);
  const pics = near.length
    ? near.map((p) => `${p.what} ${p.width}×${p.height}`).join(', ')
    : 'none';
  return `long task ${Math.round(duration)} ms at ${(start / 1000).toFixed(2)} s; last action: ${lastAction}; pictures loaded or decoded around it: ${pics}`;
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
  private maxDelay: number | null = null;
  private lastAction = 'none yet';
  private pictures: PictureEvent[] = [];

  constructor(private readonly log: (line: string) => void) {}

  start(timers: () => string[]): void {
    this.timers = timers;
    try {
      this.observer = new PerformanceObserver((list) => {
        for (const e of list.getEntries()) {
          this.longTasks += 1;
          this.longTaskMs += e.duration;
          // D-007: every long task gets its own line with what it overlapped.
          this.log(describeLongTask(e.startTime, e.duration, this.lastAction, this.pictures));
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

  /** D-008: a core → UI delay (ms) measured when the UI acted on a pad event. */
  inputDelay(ms: number): void {
    if (this.maxDelay === null || ms > this.maxDelay) this.maxDelay = ms;
  }

  /** D-007: what the user last did (for long-task lines). */
  action(text: string): void {
    this.lastAction = `${text} (${(performance.now() / 1000).toFixed(2)} s)`;
  }

  /** D-007: a picture finished loading or decoding. Keeps the last 50. */
  picture(what: string, width: number, height: number): void {
    this.pictures.push({ at: performance.now(), what, width, height });
    if (this.pictures.length > 50) this.pictures.shift();
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
    const delay = this.maxDelay;
    this.maxDelay = null;
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
        describeFrames(f, delay),
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
