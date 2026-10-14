// Menu sounds (B-1, audio model A): preloaded Web Audio buffers through AIONE's own gain stages,
// AIONE Volume × Mute × the menu category × this play. AIONE never touches the Windows volume.
//
// Sounds (B-1 rev. q): AIONE's sound set by function and place, one family in A major, made
// from FilmCow recordings by tools/sound/make_menu_family.py (docs/sound/SOUND_MAPPING.md):
// row move (tiny 8 ms click, E), drawer move (a different tick, A), row <-> drawer (C#), edge
// bump (dull, low A), confirm (F#), back (D), error (two falling bumps). 48 kHz mono, levelled
// on the loudest 30 ms (row move −24 … confirm −18 LUFS), true peaks under −1 dBFS.
//
// **One sound per action, never doubled:** every action calls `play` once; a sound name is
// monophonic (a new row move stops the previous one), so sounds never stack.
// **Repeats (B-1 rev. p, supersedes T1.1's pacing):** the row move plays on every move, every
// hold repeat included (every 90 ms), like Switch; it is tiny and soft enough for that.

import rowMoveUrl from '../../../assets/sounds/filmcow-menu/row_move.wav?url';
import drawerMoveUrl from '../../../assets/sounds/filmcow-menu/drawer_move.wav?url';
import rowDrawerUrl from '../../../assets/sounds/filmcow-menu/row_drawer.wav?url';
import edgeBumpUrl from '../../../assets/sounds/filmcow-menu/edge_bump.wav?url';
import confirmUrl from '../../../assets/sounds/filmcow-menu/confirm.wav?url';
import backUrl from '../../../assets/sounds/filmcow-menu/back.wav?url';
import errorUrl from '../../../assets/sounds/filmcow-menu/error.wav?url';

export type SoundName = 'rowMove' | 'drawerMove' | 'rowDrawer' | 'edgeBump' | 'confirm' | 'back' | 'error';

/** AIONE's sounds by function and place (B-1 rev. q), made from FilmCow recordings by
 *  tools/sound/make_menu_family.py (one family in A major; docs/sound/SOUND_MAPPING.md). */
export const SOUND_FILES: Record<SoundName, { file: string; source: string; url: string }> = {
  rowMove: { file: 'row_move.wav', source: 'FilmCow "clicky button 7", cut to 8 ms (your move B)', url: rowMoveUrl },
  drawerMove: { file: 'drawer_move.wav', source: 'FilmCow "mouse click 2"', url: drawerMoveUrl },
  rowDrawer: { file: 'row_drawer.wav', source: 'FilmCow "metal latches 2"', url: rowDrawerUrl },
  edgeBump: { file: 'edge_bump.wav', source: 'FilmCow "wood hit 4", low and dull', url: edgeBumpUrl },
  confirm: { file: 'confirm.wav', source: 'FilmCow "mechanical button 8" + "glass ding 4" (your select A)', url: confirmUrl },
  back: { file: 'back.wav', source: 'FilmCow "metal switch 1" (your back B)', url: backUrl },
  error: { file: 'error.wav', source: 'FilmCow "wood hit 4", two falling bumps', url: errorUrl },
};

export const GAINS = {
  /** AIONE Volume and Mute (settings in T2 / T3). */
  volume: 1,
  muted: false,
  /** Menu category: 0 dB; the files carry the family's levels. */
  menu: 1,
};

export interface PlayLog {
  (line: string): void;
}

export class MenuSounds {
  private ctx: AudioContext | null = null;
  private master: GainNode | null = null;
  private menu: GainNode | null = null;
  private buffers = new Map<SoundName, AudioBuffer>();
  private voices = new Map<SoundName, AudioBufferSourceNode>();

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
      this.log(`menu sounds ready: ${this.buffers.size} sounds (FilmCow family: row move, drawer move, row-drawer, edge bump, confirm, back, error) decoded in ${ms} ms; audio ${ctx.state}`);
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

  /** Plays one sound for one action (B-1 rev. p: every move, every repeat included; a sound
   *  never stacks on itself). Returns 'played' or 'unavailable' (not loaded), for the log. */
  play(name: SoundName): 'played' | 'unavailable' {
    const ctx = this.ctx;
    const buf = this.buffers.get(name);
    if (!ctx || !buf || !this.menu) return 'unavailable';
    this.voices.get(name)?.stop();
    const src = ctx.createBufferSource();
    src.buffer = buf;
    const g = ctx.createGain();
    g.gain.value = 1;
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
