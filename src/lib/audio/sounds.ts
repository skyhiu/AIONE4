// Menu sounds (B-1, audio model A): preloaded Web Audio buffers through AIONE's own gain stages,
// AIONE Volume × Mute × the menu category × this play. AIONE never touches the Windows volume.
//
// Sounds (B-1 rev. h / j, alpha.5): AIONE's own menu family made from FilmCow recordings by
// tools/sound/make_menu_family.py (docs/sound/SOUND_MAPPING.md): move (a clicky button, tuned to
// E), select (a mechanical button with a little glass, F#: a step above), back (a metal switch,
// D: a step below). 48 kHz mono; each levelled to the family (loudest 30 ms: move −20, select
// −18, back −19 LUFS), true peaks under −1 dBFS.
//
// **One sound per action, never doubled:** every action calls `play` once; a sound name is
// monophonic (a new move stops the previous move), so sounds never stack.
// **Repeats (holding a direction):** the first press always plays. While the direction repeats
// (every 90 ms), the move sound plays at most every 180 ms (every second repeat, about 5.5 per
// second) and 3 dB softer, so a long hold ticks evenly without becoming a buzz.

import moveUrl from '../../../assets/sounds/filmcow-menu/move.wav?url';
import selectUrl from '../../../assets/sounds/filmcow-menu/select.wav?url';
import backUrl from '../../../assets/sounds/filmcow-menu/back.wav?url';

export type SoundName = 'move' | 'select' | 'back';

export const SOUND_FILES: Record<SoundName, { file: string; source: string; url: string }> = {
  move: { file: 'move.wav', source: 'FilmCow "clicky button 7" (move B)', url: moveUrl },
  select: { file: 'select.wav', source: 'FilmCow "mechanical button 8" + "glass ding 4" (select A)', url: selectUrl },
  back: { file: 'back.wav', source: 'FilmCow "metal switch 1" (back B)', url: backUrl },
};

/** Gain stages (AIONE's own, model A). Volume and mute become settings in T2 / T3. */
export const GAINS = {
  volume: 1,
  muted: false,
  /** Menu category: 0 dB; the files already carry the family's level (alpha.5). */
  menu: 1,
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

  /** Creates the context and decodes the three sounds (once), each on its own: one failing
   *  sound never silences the others (D-009). Logs the result, and for a failure the address
   *  and the reason. */
  async init(): Promise<void> {
    if (this.ctx) return;
    const started = performance.now();
    let ctx: AudioContext;
    try {
      ctx = new AudioContext({ latencyHint: 'interactive' });
    } catch (err) {
      this.log(`menu sounds unavailable: no audio context (${String(err)})`);
      return;
    }
    this.ctx = ctx;
    this.master = ctx.createGain();
    this.master.gain.value = GAINS.muted ? 0 : GAINS.volume;
    this.menu = ctx.createGain();
    this.menu.gain.value = GAINS.menu;
    this.menu.connect(this.master);
    this.master.connect(ctx.destination);
    const names = Object.keys(SOUND_FILES) as SoundName[];
    const failures: string[] = [];
    await Promise.all(
      names.map(async (n) => {
        const url = SOUND_FILES[n].url;
        try {
          const res = await fetch(url);
          if (!res.ok) throw new Error(`HTTP ${res.status}`);
          this.buffers.set(n, await ctx.decodeAudioData(await res.arrayBuffer()));
        } catch (err) {
          const shown = url.startsWith('data:') ? `${url.slice(0, 30)}… (inlined)` : url;
          failures.push(`${n} from ${shown}: ${String(err)}`);
        }
      }),
    );
    const ms = Math.round(performance.now() - started);
    if (failures.length === 0) {
      this.log(`menu sounds ready: ${this.buffers.size} sounds (FilmCow family: move, select, back) decoded in ${ms} ms; audio ${ctx.state}`);
    } else {
      this.log(
        `menu sounds ${this.buffers.size ? 'partly ' : ''}unavailable: ${this.buffers.size} of ${names.length} loaded in ${ms} ms; failed: ${failures.join('; ')}`,
      );
    }
  }

  /** Resumes a suspended context (a browser without the autoplay flag); harmless otherwise. */
  resume(): void {
    if (this.ctx?.state === 'suspended') void this.ctx.resume();
  }

  /** Plays one sound for one action. Returns what really happened, for the log line:
   *  'played', 'paced' (a repeat inside the 180 ms pacing) or 'unavailable' (not loaded). */
  play(name: SoundName, kind: 'press' | 'repeat' = 'press'): 'played' | 'paced' | 'unavailable' {
    const ctx = this.ctx;
    const buf = this.buffers.get(name);
    if (!ctx || !buf || !this.menu) return 'unavailable';
    if (name === 'move' && !this.pacer.shouldPlay(kind, performance.now())) return 'paced';
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
    return 'played';
  }

  get state(): string {
    return this.ctx?.state ?? 'none';
  }
}
