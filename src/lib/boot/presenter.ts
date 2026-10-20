// Plays one Boot Up or Boot Down presentation (reference A9; B-5 final; T1.2).
//
// - Default ("Window B"): the boot sound is decoded and scheduled on its own Web Audio clock; every
//   frame is drawn at the position the listener HEARS (the sound clock minus the output latency),
//   so the clamp's hit and the clamp frame meet (the first frame past 2550 ms is logged).
// - Custom video: the user's video (AIONE 4's copy), with its own sound; a black hold of at most
//   4 s for it to start, then one controlled fallback to the default (the caller decides).
// - Foreground loss (A9, L-02): both pause (the audio clock stops; the video pauses) and resume
//   in sync; the start timeout does not run while AIONE is in the background.
// - Skip (Boot Up only): ends at once, stops the sound, no fallback sound.

import { BOOT_DOWN_MS, BOOT_UP_MS, CLAMP_MS, createWindowB } from './windowB';

/** The Boot Down sound is 2.6 s; the screen stays black until it ends (B-5 final). */
export const BOOT_DOWN_SOUND_MS = 2600;
export const CUSTOM_START_TIMEOUT_MS = 4000;
export const CUSTOM_DOWN_CAP_MS = 15000;

export type Outcome = 'Completed' | 'Skipped' | 'Stopped' | `Failed: ${string}`;
type Log = (level: 'info' | 'warn' | 'error', line: string) => void;

interface FrameStats {
  frames: number;
  longest: number;
  over33: number;
  /** D-013: each frame over 33 ms with its place in the timeline (ms) and its length. */
  slow: { at: number; dt: number }[];
}

/** Window B's fade to black begins here (B-5 final: 4.65–5.00 s). */
export const FADE_START_MS = 4650;

export class BootPresenter {
  /**
   * D-049 (the audio model): AIONE's own level for the Boot Up / Boot Down sound and videos
   * (Volume × Mute, from the settings; never Windows' volume). Set before any presentation.
   */
  static level = 1;
  static levelLine = 'AIONE volume 100 %, unmuted';
  static setLevel(volume: number, muted: boolean) {
    BootPresenter.level = muted ? 0 : Math.max(0, Math.min(1, volume));
    BootPresenter.levelLine = `AIONE volume ${Math.round(volume * 100)} %, ${muted ? 'muted' : 'unmuted'}`;
  }
  private raf = 0;
  private ctx: AudioContext | null = null;
  private source: AudioBufferSourceNode | null = null;
  private video: HTMLVideoElement | null = null;
  private finish: ((o: Outcome) => void) | null = null;
  private paused = false;
  private last = 0;
  private stats: FrameStats = { frames: 0, longest: 0, over33: 0, slow: [] };
  /** D-016: every timer of the current presentation, cancelled when it ends. */
  private timers: ReturnType<typeof setTimeout>[] = [];
  /** Each presentation's timers and listeners act only while it is the current one. */
  private gen = 0;

  constructor(
    private readonly host: HTMLElement,
    private readonly log: Log,
    // B-5 (rev. o): the brand name in M PLUS Rounded 1c everywhere.
    private readonly fontFamily = "'M PLUS Rounded 1c', 'Segoe UI', sans-serif",
  ) {}

  /** A timer that belongs to the current presentation (D-016). */
  private later(fn: () => void, ms: number): void {
    const gen = this.gen;
    this.timers.push(
      setTimeout(() => {
        if (gen === this.gen) fn();
      }, ms),
    );
  }

  /** The default Window B presentation with its sound. `onFade` runs once when the fade to
   *  black begins (D-014: Home's first frames are prepared while the screen is still black). */
  async playDefault(which: 'up' | 'down', soundUrl: string, onFade?: () => void): Promise<Outcome> {
    this.clear();
    const wb = createWindowB(this.fontFamily);
    wb.svg.setAttribute('data-testid', `boot-${which}-default`);
    this.host.appendChild(wb.svg);
    const frame = which === 'up' ? wb.bootUpFrame : (t: number) => wb.bootDownFrame(Math.min(t, BOOT_DOWN_MS));
    frame(0);
    const total = which === 'up' ? BOOT_UP_MS : BOOT_DOWN_SOUND_MS;
    const label = which === 'up' ? 'Boot Up' : 'Boot Down';
    /** The heard position (ms of the sound) at a page time `at` (performance.now() ms). */
    let clock: (at: number) => number;
    let latency = 0;
    let clockNote = 'frame clock (no sound)';
    try {
      const ctx = new AudioContext({ latencyHint: 'interactive' });
      this.ctx = ctx;
      const res = await fetch(soundUrl);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const buf = await ctx.decodeAudioData(await res.arrayBuffer());
      const gain = ctx.createGain();
      gain.gain.value = BootPresenter.level; // D-049: AIONE's Volume × Mute
      this.log('info', `boot sound: ${BootPresenter.levelLine} (AIONE's own; Windows' volume and mute untouched)`);
      gain.connect(ctx.destination);
      const src = ctx.createBufferSource();
      src.buffer = buf;
      src.connect(gain);
      this.source = src;
      if (ctx.state === 'suspended') await ctx.resume();
      const startAt = ctx.currentTime + 0.06;
      src.start(startAt);
      latency = ((ctx as AudioContext & { outputLatency?: number }).outputLatency || ctx.baseLatency || 0) * 1000;
      // D-015: the output timestamp says which sample is heard at which page time; between its
      // updates the position is interpolated with the page clock (sub-millisecond), instead of
      // the audio clock's 8–11 ms blocks. Fallback: currentTime minus the output latency.
      clock = (at: number) => {
        const ts = ctx.getOutputTimestamp?.();
        if (ts && ts.contextTime && ts.performanceTime) {
          clockNote = 'audio output timestamp, interpolated with the page clock (resolution about 0.01 ms)';
          return (ts.contextTime + (at - ts.performanceTime) / 1000 - startAt) * 1000;
        }
        clockNote = `audio currentTime minus output latency (resolution: the audio block, about ${((128 / ctx.sampleRate) * 1000).toFixed(1)} ms or more)`;
        return (ctx.currentTime - startAt) * 1000 - latency;
      };
      this.log('info', `${label} (Window B): sound ${Math.round(buf.duration * 1000)} ms decoded; audio ${ctx.state}; output latency ${latency.toFixed(1)} ms`);
    } catch (err) {
      // Never stuck: without sound the animation runs on the frame clock.
      const t0 = performance.now();
      clock = (at: number) => (this.paused ? this.last : at - t0);
      this.log('warn', `${label} (Window B): sound unavailable (${String(err)}); animation without sound`);
    }
    return new Promise<Outcome>((resolve) => {
      this.finish = resolve;
      let clampLogged = which === 'down';
      let faded = false;
      let prev = performance.now();
      let prevT = 0;
      let interval = 1000 / 60;
      const gen = this.gen;
      const step = (now: number) => {
        if (gen !== this.gen) return;
        const dt = now - prev;
        // The refresh interval, smoothed (60 Hz → 16.7 ms); ignore pauses and hitches.
        if (dt > 4 && dt < 40) interval = interval * 0.9 + dt * 0.1;
        prev = now;
        // D-015: draw the frame for the refresh it will be shown on (the next one).
        const t = Math.max(0, clock(now + interval));
        this.last = t;
        this.count(dt, t);
        frame(t);
        if (!clampLogged && t >= CLAMP_MS) {
          clampLogged = true;
          this.log(
            'info',
            `Boot Up clamp: frame drawn at ${t.toFixed(1)} ms of the heard sound, for the refresh predicted ${interval.toFixed(1)} ms ahead (target ${CLAMP_MS} ms, ${(t - CLAMP_MS).toFixed(1)} ms after; previous frame at ${prevT.toFixed(1)} ms, frame interval ${dt.toFixed(1)} ms; clock: ${clockNote}; output latency ${latency.toFixed(1)} ms)`,
          );
        }
        if (!faded && which === 'up' && t >= FADE_START_MS) {
          faded = true;
          onFade?.();
        }
        prevT = t;
        if (t >= total) return this.end('Completed', label);
        this.raf = requestAnimationFrame(step);
      };
      this.raf = requestAnimationFrame(step);
    });
  }

  /** A custom video. Resolves 'Failed: …' before it starts (the caller falls back) or after. */
  playVideo(which: 'up' | 'down', url: string, opts: { muted: boolean; capMs?: number | undefined }): Promise<Outcome> {
    this.clear();
    const label = which === 'up' ? 'custom Boot Up' : 'custom Boot Down';
    const v = document.createElement('video');
    v.setAttribute('data-testid', `boot-${which}-video`);
    v.playsInline = true;
    v.preload = 'auto';
    // D-049: AIONE's Volume × Mute on the custom video too.
    v.muted = opts.muted || BootPresenter.level === 0;
    v.volume = BootPresenter.level;
    this.log('info', `${label} sound: ${BootPresenter.levelLine} (AIONE's own; Windows' volume and mute untouched)`);
    v.style.cssText = 'position:absolute;inset:0;width:100%;height:100%;object-fit:contain;background:#000';
    this.video = v;
    this.host.appendChild(v);
    const gen = this.gen;
    const mine = () => gen === this.gen;
    return new Promise<Outcome>((resolve) => {
      this.finish = resolve;
      let started = false;
      let waited = 0;
      let lastCheck = performance.now();
      const startTimer = () => {
        // The start timeout counts only while AIONE is in front (A9).
        const now = performance.now();
        if (!this.paused) waited += now - lastCheck;
        lastCheck = now;
        if (started || !this.finish || !mine()) return;
        if (waited >= CUSTOM_START_TIMEOUT_MS) return this.end(`Failed: did not start within ${CUSTOM_START_TIMEOUT_MS / 1000} s`, label);
        this.later(startTimer, 100);
      };
      this.later(startTimer, 100);
      let prev = performance.now();
      const tick = (now: number) => {
        this.count(now - prev, v.currentTime * 1000);
        prev = now;
        if (!mine()) return;
        if (opts.capMs && v.currentTime * 1000 >= opts.capMs) return this.end('Completed', label);
        if (this.finish) this.raf = requestAnimationFrame(tick);
      };
      v.addEventListener('playing', () => {
        if (started || !mine()) return;
        started = true;
        this.log('info', `${label}: playing (${v.videoWidth}×${v.videoHeight}, ${v.duration.toFixed(2)} s)`);
        this.raf = requestAnimationFrame(tick);
        this.later(() => {
          const decoded = (v as HTMLVideoElement & { webkitAudioDecodedByteCount?: number }).webkitAudioDecodedByteCount;
          this.log(
            'info',
            `${label}: after the first frames: volume ${v.volume.toFixed(2)}, muted ${v.muted}, audio decoded ${decoded ?? 'unknown'} bytes (sound ${decoded === undefined ? 'unknown' : decoded > 0 ? 'yes' : 'no'})`,
          );
        }, 700);
      });
      v.addEventListener('ended', () => mine() && this.end('Completed', label));
      v.addEventListener('error', () => mine() && this.end(`Failed: ${v.error?.message || `media error ${v.error?.code ?? '?'}`}${started ? ' (mid-playback)' : ''}`, label));
      v.src = url;
      v.play().catch((err: unknown) => {
        if (!started && mine()) this.log('warn', `${label}: play() refused (${String(err)})`);
      });
      if (which === 'down') this.later(() => this.end('Completed', `${label} (cap ${CUSTOM_DOWN_CAP_MS / 1000} s)`), CUSTOM_DOWN_CAP_MS);
    });
  }

  /** Tests (mocked build only): draw one moment of the default animation and hold it. */
  showFrame(which: 'up' | 'down', t: number): void {
    this.clear();
    const wb = createWindowB(this.fontFamily);
    wb.svg.setAttribute('data-testid', `boot-${which}-default`);
    this.host.appendChild(wb.svg);
    (which === 'up' ? wb.bootUpFrame : wb.bootDownFrame)(t);
  }

  /** Boot Up only: ends at once, sound stopped, no fallback sound. */
  skip(): void {
    this.end('Skipped', 'Boot Up');
  }

  /** Stops whatever plays (e.g. Close during Boot Up), silently. */
  stop(): void {
    this.end('Stopped', 'presentation');
  }

  /** A9 / L-02: AIONE lost or regained the foreground. */
  setForeground(front: boolean): void {
    if (this.paused === !front || !this.finish) return;
    this.paused = !front;
    if (this.ctx) void (front ? this.ctx.resume() : this.ctx.suspend());
    if (this.video) {
      const at = this.video.currentTime.toFixed(2);
      if (front) void this.video.play().catch(() => undefined);
      else this.video.pause();
      this.log('info', `boot ${front ? 'resumed' : 'paused'} (AIONE ${front ? 'back in front' : 'in the background'}) at video ${at} s`);
    } else if (this.ctx) {
      this.log('info', `boot ${front ? 'resumed' : 'paused'} (AIONE ${front ? 'back in front' : 'in the background'}) at ${this.last.toFixed(0)} ms`);
    }
  }

  get running(): boolean {
    return this.finish !== null;
  }

  private count(dt: number, at: number): void {
    if (this.stats.frames > 0 && !this.paused) {
      this.stats.longest = Math.max(this.stats.longest, dt);
      if (dt > 33) {
        this.stats.over33 += 1;
        this.stats.slow.push({ at, dt });
      }
    }
    this.stats.frames += 1;
  }

  private end(outcome: Outcome, label: string): void {
    const done = this.finish;
    if (!done) return;
    this.finish = null;
    // D-016: nothing of this presentation runs after its outcome.
    this.gen += 1;
    this.timers.forEach((id) => clearTimeout(id));
    this.timers = [];
    cancelAnimationFrame(this.raf);
    try {
      this.source?.stop();
    } catch {
      // already ended
    }
    this.video?.pause();
    void this.ctx?.close();
    const s = this.stats;
    this.log(
      outcome.startsWith('Failed') ? 'warn' : 'info',
      `${label}: ${outcome}; ${s.frames} frames, longest ${s.longest.toFixed(1)} ms, over 33 ms: ${s.over33}${
        s.slow.length
          ? ` (at ${s.slow
              .slice(0, 12)
              .map((x) => `${(x.at / 1000).toFixed(2)} s: ${x.dt.toFixed(1)} ms`)
              .join(', ')}${s.slow.length > 12 ? `, and ${s.slow.length - 12} more` : ''})`
          : ''
      }`,
    );
    this.ctx = null;
    this.source = null;
    done(outcome);
  }

  private clear(): void {
    this.gen += 1;
    // Release a previous video player (A9: never two presentations, no orphaned player).
    if (this.video) {
      this.video.pause();
      this.video.removeAttribute('src');
      this.video.load();
    }
    this.host.replaceChildren();
    this.video = null;
    this.stats = { frames: 0, longest: 0, over33: 0, slow: [] };
    this.paused = false;
  }
}
