// Menu sounds (B-1, audio model A): preloaded Web Audio buffers through AIONE's own gain stages,
// AIONE Volume × Mute × the menu category × this play. AIONE never touches the Windows volume.
//
// Mapping (docs/sound/SOUND_MAPPING.md, a proposal for the user to judge): move = #24,
// select = #09, back = #15 from "50 Menu Interface SFX" by ColorAlpha (CC BY 4.0), unchanged.
//
// **One sound per action, never doubled:** every action calls `play` once; a sound name is
// monophonic (a new move stops the previous move), so sounds never stack.
// **Repeats (holding a direction):** the first press always plays. While the direction repeats
// (every 90 ms), the move sound plays at most every 180 ms (every second repeat, about 5.5 per
// second) and 3 dB softer, so a long hold ticks evenly without becoming a buzz.

import moveUrl from '../../../assets/sounds/50-menu-interface-sfx/MI_SFX 24.wav?url';
import selectUrl from '../../../assets/sounds/50-menu-interface-sfx/MI_SFX 09.wav?url';
import backUrl from '../../../assets/sounds/50-menu-interface-sfx/MI_SFX 15.wav?url';

export type SoundName = 'move' | 'select' | 'back';

export const SOUND_FILES: Record<SoundName, { pack: number; url: string }> = {
  move: { pack: 24, url: moveUrl },
  select: { pack: 9, url: selectUrl },
  back: { pack: 15, url: backUrl },
};

/** Gain stages (AIONE's own, model A). Volume and mute become settings in T2 / T3. */
export const GAINS = {
  volume: 1,
  muted: false,
  /** Menu category: about −4.4 dB; the pack is peak-normalised to 0 dBFS. */
  menu: 0.6,
  /** A repeat's move sound: −3 dB. */
  repeat: 0.7,
};

export const REPEAT_MIN_GAP_MS = 180;

/** Decides whether a move sound plays (pure; tested). */
export class RepeatPacer {
  private last = -Infinity;
  constructor(private readonly minGap = REPEAT_MIN_GAP_MS) {}
  shouldPlay(kind: 'press' | 'repeat', now: number): boolean {
    if (kind === 'press' || now - this.last >= this.minGap) {
      this.last = now;
      return true;
    }
    return false;
  }
}

export interface PlayLog {
  (line: string): void;
}

export class MenuSounds {
  private ctx: AudioContext | null = null;
  private master: GainNode | null = null;
  private menu: GainNode | null = null;
  private buffers = new Map<SoundName, AudioBuffer>();
  private voices = new Map<SoundName, AudioBufferSourceNode>();
  private pacer = new RepeatPacer();

  constructor(private readonly log: PlayLog = () => {}) {}

  /** Creates the context and decodes the three sounds (once). Logs the result. */
  async init(): Promise<void> {
    if (this.ctx) return;
    const started = performance.now();
    try {
      const ctx = new AudioContext({ latencyHint: 'interactive' });
      this.ctx = ctx;
      this.master = ctx.createGain();
      this.master.gain.value = GAINS.muted ? 0 : GAINS.volume;
      this.menu = ctx.createGain();
      this.menu.gain.value = GAINS.menu;
      this.menu.connect(this.master);
      this.master.connect(ctx.destination);
      const names = Object.keys(SOUND_FILES) as SoundName[];
      await Promise.all(
        names.map(async (n) => {
          const res = await fetch(SOUND_FILES[n].url);
          const buf = await ctx.decodeAudioData(await res.arrayBuffer());
          this.buffers.set(n, buf);
        }),
      );
      this.log(
        `menu sounds ready: ${this.buffers.size} sounds (move #24, select #09, back #15) decoded in ${Math.round(
          performance.now() - started,
        )} ms; audio ${ctx.state}`,
      );
    } catch (err) {
      this.log(`menu sounds unavailable: ${String(err)}`);
    }
  }

  /** Resumes a suspended context (a browser without the autoplay flag); harmless otherwise. */
  resume(): void {
    if (this.ctx?.state === 'suspended') void this.ctx.resume();
  }

  /** Plays one sound for one action. For moves, `kind` decides the repeat pacing. Returns
   *  whether it played (for the input log line). */
  play(name: SoundName, kind: 'press' | 'repeat' = 'press'): boolean {
    if (name === 'move' && !this.pacer.shouldPlay(kind, performance.now())) return false;
    const ctx = this.ctx;
    const buf = this.buffers.get(name);
    if (!ctx || !buf || !this.menu) return false;
    this.voices.get(name)?.stop();
    const src = ctx.createBufferSource();
    src.buffer = buf;
    const g = ctx.createGain();
    g.gain.value = kind === 'repeat' ? GAINS.repeat : 1;
    src.connect(g);
    g.connect(this.menu);
    src.onended = () => {
      if (this.voices.get(name) === src) this.voices.delete(name);
      src.disconnect();
      g.disconnect();
    };
    this.voices.set(name, src);
    src.start();
    return true;
  }

  get state(): string {
    return this.ctx?.state ?? 'none';
  }
}
