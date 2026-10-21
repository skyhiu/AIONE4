<script lang="ts">
  // AIONE 4 (T1.1): Home with the copy dialog over it when needed. This component owns input
  // (A6): every controller, keyboard and mouse event arrives here once, becomes one action, and
  // goes to the active context only: the dialog when it is open, else Home.
  import { onMount, untrack } from 'svelte';
  import { loadCore } from './lib/core';
  import type { AppInfo, Core, GameView, LibraryLoad, PcStatus, Unlisten } from './lib/core/types';
  import {
    countsAsHomeInput,
    inputWhere,
    describeInput,
    describeKeyRelease,
    fromKey,
    fromPadEvent,
    toAction,
    type InputEvent,
    type UiAction,
  } from './lib/input/events';
  import { applyPadConnection, applyPadSlots, initialPads } from './lib/input/pads';
  import { BUTTON_COPY, BUTTON_SKIP, choose, describeCounts, describeSkipped, flowAction, flowDone, flowProgress, initialFlow, type FlowEffect, type FlowState } from './lib/migrationFlow';
  import { formatBytes, plural } from './lib/format';
  import { isBlockedBrowserKey, isRestartShortcut } from './lib/input/shortcuts';
  import { KeyRepeater, type ReleaseInfo } from './lib/input/keyRepeat';
  import { MenuSounds, type SoundRequest } from './lib/audio/sounds';
  import { PerfLog } from './lib/perf/perf';
  import { deviceAfter, type GlyphDevice } from './lib/frame/glyphs';
  import {
    clickTile,
    hoverTile,
    initialHome,
    move,
    toPadMode,
    visibleTiles,
    wheel,
    withCount,
    withVisible,
    wrapRow,
    DRAWER,
    hoverDrawer,
    moveDrawer,
    moveZone,
    type HomeState,
  } from './lib/home/model';
  import Home from './lib/home/Home.svelte';
  import { BootPresenter, type Outcome } from './lib/boot/presenter';
  import { playBump } from './lib/focus/bump';
  import QuickMenu, { POWER_ITEMS, QM_ROWS } from './lib/components/QuickMenu.svelte';
  import PowerOptions from './lib/components/PowerOptions.svelte';
  import bootUpSound from '../assets/sounds/boot/aione_boot_up.wav?url';
  import bootDownSound from '../assets/sounds/boot/aione_boot_down.wav?url';
  import type { BootMedia } from './lib/core/types';
  import CopyDialog from './lib/components/CopyDialog.svelte';
  import ConfirmDialog from './lib/components/ConfirmDialog.svelte';

  /** Mouse movement that counts as deliberate (A7): beyond this many CSS pixels. */
  const MOUSE_THRESHOLD_PX = 6;
  const params = new URLSearchParams(globalThis.location?.search ?? '');

  let core: Core | null = null;
  /** The lifecycle (A9): black hold → Boot Up → Home; Boot Down before closing. */
  let phase = $state<'black' | 'boot' | 'home' | 'bootdown'>('black');
  let bootHost: HTMLDivElement | undefined = $state();
  let presenter: BootPresenter | null = null;
  /** D-014: Home is mounted under the black boot layer before it is shown, so its first
   *  frames (layout, pictures) are drawn while the screen is still black. */
  let homeEarly = $state(false);
  /** D-014: when Home finished its first frames under the black screen (performance.now()). */
  let homeReadyAt = 0;
  let bootStartedAt = 0;
  let bootMedia: BootMedia | null = null;
  /** Icons (Material Symbols) are shown only once their font is really there; without it the
   *  browser would draw the ligature names ("arrow_drop_down") as large text (alpha.7 CI). */
  let iconsReady = $state(false);
  const ICON_FONT = '400 24px "Material Symbols Rounded"';
  const ICON_SAMPLE = 'arrow_drop_down sports_esports wifi wifi_off lan battery_android_frame_full';
  async function loadFonts(c: Core): Promise<void> {
    const started = performance.now();
    try {
      await Promise.all([
        document.fonts.load(ICON_FONT, ICON_SAMPLE),
        document.fonts.load('400 24px "M PLUS Rounded 1c"', 'AIONE'),
        document.fonts.load('500 24px "M PLUS Rounded 1c"', 'AIONE'),
      ]);
    } catch (err) {
      c.log('error', `fonts: loading failed (${String(err)})`);
    }
    iconsReady = document.fonts.check(ICON_FONT, ICON_SAMPLE);
    c.log(
      iconsReady ? 'info' : 'error',
      iconsReady
        ? `fonts ready in ${Math.round(performance.now() - started)} ms (Material Symbols Rounded, M PLUS Rounded 1c)`
        : 'fonts: Material Symbols Rounded is not available; icons stay hidden rather than shown as text',
    );
    if (!iconsReady) {
      // A late arrival still shows them.
      document.fonts.addEventListener('loadingdone', () => {
        if (!iconsReady && document.fonts.check(ICON_FONT, ICON_SAMPLE)) {
          iconsReady = true;
          c.log('info', 'fonts: Material Symbols Rounded arrived late; icons shown');
        }
      });
    }
  }

  /** The black hold is bounded (reference A9: at most 4 s). */
  const BLACK_HOLD_MAX_MS = 4000;
  let info = $state<AppInfo | null>(null);
  let pads = $state(initialPads());
  let status = $state<PcStatus | null>(null);
  let flow = $state<FlowState>({ phase: 'hidden' });
  let library = $state<LibraryLoad | null>(null);
  let loadState = $state<'loading' | 'loaded' | 'empty' | 'error'>('loading');
  /** Home shows the games that are not hidden. */
  let games = $state<GameView[]>([]);
  let home = $state<HomeState>(initialHome(0, visibleTiles(window.innerWidth, window.innerHeight)));
  /** A7: 'pad' = controller / keyboard (focus ring), 'mouse' = hover only. */
  let inputMode = $state<'pad' | 'mouse'>('pad');
  let device = $state<GlyphDevice>('xbox');
  let pressed = $state<number | null>(null);
  let pressTimer: ReturnType<typeof setTimeout> | null = null;
  /** B-15 (rev. t): the extras are on unless this run says `--extras=none`. */
  const extrasOn = $derived(!(info?.extras ?? []).includes('none'));
  let mouseTravel = 0;

  const dialogOpen = $derived(flow.phase !== 'hidden');
  // Tests: a fixed clock (?time=12:24&h12=1) so screenshots are stable.
  const fixedTime = (() => {
    const t = params.get('time');
    if (!t || import.meta.env.MODE !== 'mock') return null;
    const [h, m] = t.split(':').map(Number);
    const d = new Date(2026, 9, 13, h ?? 0, m ?? 0);
    return d;
  })();
  const hour12 = params.get('h12') === null ? undefined : params.get('h12') === '1';

  const sounds = new MenuSounds((line) => core?.log('info', line));
  /** B-24 / the audio model: AIONE's own Volume, Mute and menu level → its gain stages, live. */
  async function loadSettings(c: Core) {
    try {
      const st = await c.getSettings();
      sounds.setLevels(st.volume / 100, st.muted, st.menuSounds ? st.menuSoundVolume / 100 : 0);
      BootPresenter.setLevel(st.volume / 100, st.muted); // D-049: Boot Up / Boot Down too
      uiVolume = st.volume;
      uiMuted = st.muted;
      c.log('info', `audio: AIONE volume ${st.volume} %, ${st.muted ? 'muted' : 'unmuted'}, menu sounds ${st.menuSounds ? `${st.menuSoundVolume} %` : 'off'} (AIONE's own; Windows' volume and mute untouched)`);
    } catch (err) {
      c.log('error', `settings: couldn't load (${String(err)}); defaults in effect`);
    }
    if ('__aioneMock' in window) (window as unknown as { __aioneSoundLevels: () => unknown }).__aioneSoundLevels = () => sounds.levels();
  }
  const perf = new PerfLog((line) => core?.log('info', line));
  let firstKeyLogged = false;
  // D-006: AIONE times key repeats itself (400 / 90 ms); only navigation keys repeat.
  const keys = new KeyRepeater((e, phase, release) => onKeyPhase(fromKey(e, phase), release));

  /** QA-6 (alpha.9): a game as "focus N/M", an empty slot as "empty slot N". */
  function where(i: number): string {
    return i < games.length ? `focus ${i + 1}/${games.length}` : `empty slot ${i - games.length + 1} (after ${games.length} games)`;
  }

  /** The log wording for one sound request (QA-6: says what really happened). */
  function play(name: SoundRequest, gainDb = 0): string {
    return sounds.play(name, gainDb) === 'played' ? `sound ${name}${gainDb ? ` (${gainDb} dB)` : ''}` : 'no sound (sounds unavailable)';
  }

  /** B-14 (rev. q): a blocked move. The focus ring springs toward the pressed direction and back
   *  (the tile or button stays still), with the edge-bump sound. A held direction bumps once:
   *  its repeats at the same edge do nothing. */
  let bump = $state<{ dir: 'left' | 'right' | 'up' | 'down'; n: number; zone: string } | null>(null);
  /** D-019: the directions that already bumped in their current hold (one memory per
   *  direction, cleared when that direction is released). */
  const bumpedHolds: Record<string, boolean> = {};
  /** B-14 (rev. q / s / u): a blocked move bumps the ring only, with the edge-bump sound (instead
   *  of the click, never with it); once per held direction. */
  function edge(dir: 'left' | 'right' | 'up' | 'down', kind: 'press' | 'repeat', what: string, zone: string = home.zone): string {
    if (kind === 'repeat' && bumpedHolds[dir]) return `${what}, held: no bump`;
    bumpedHolds[dir] = true;
    bump = { dir, n: (bump?.n ?? 0) + 1, zone };
    // Home's own zones keep their verified wording; the others name themselves.
    const where = zone === 'row' || zone === 'drawer' ? '' : ` (${zone})`;
    return `${what}, bump ${dir}${where}, ${play('edgeBump')}`;
  }
  // B-29: every zone but Home's own (row, drawer: B-14 in Home) is played by the shared module.
  $effect(() => {
    const b = bump;
    if (!b || b.zone === 'row' || b.zone === 'drawer') return;
    untrack(() => playBump(b.zone, b.dir, b.n));
  });
  /** The row whips to its other end in 200 ms (its normal slide is 170 ms). */
  let wrapping = $state(false);
  let wrapTimer: ReturnType<typeof setTimeout> | null = null;
  /** B-1 (rev. s): during a hold the row-move repeats get quieter after the first few. */
  let holdRepeats = 0;
  const QUIET_AFTER = 3;
  const QUIET_DB = -4;

  /**
   * D-045 (B-22 rev. ao): only keys that act in AIONE count as "held" (never media keys, Alt or
   * other modifiers), and every one's press AND release is reported to the core.
   */
  const ACTING_KEYS = new Set(['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'Enter', ' ', 'Space', 'Escape', 'Backspace', 'Tab']);
  function reportKey(ev: InputEvent) {
    if (ev.source === 'keyboard' && ACTING_KEYS.has(ev.control)) void core?.noteKey(ev.control, ev.kind !== 'up');
  }

  /** B-28: the Home key inside AIONE, decided like Guide (short at its release; held at 0.5 s). */
  let homeTimer: ReturnType<typeof setTimeout> | null = null;
  let homeFired = false;
  function onKeyPhase(ev: InputEvent, release?: ReleaseInfo) {
    if (ev.control === 'Home') {
      if (ev.kind === 'down') {
        homeFired = false;
        if (homeTimer) clearTimeout(homeTimer);
        homeTimer = setTimeout(() => {
          homeTimer = null;
          homeFired = true;
          core?.log('info', 'Home held 500 ms: Quick Menu');
          handleInput({ ...ev, kind: 'held' });
        }, 500);
      } else if (ev.kind === 'up') {
        if (homeTimer) {
          clearTimeout(homeTimer);
          homeTimer = null;
        }
        if (!homeFired) handleInput({ ...ev, kind: 'down' });
      }
      return;
    }
    if (ev.kind === 'up') {
      reportKey(ev); // D-045: the release reaches the core too (it used to stop here)
      if (release) core?.log('info', describeKeyRelease(ev.control, release.heldMs, release.repeats));
      return;
    }
    handleInput(ev);
  }

  function setMode(next: 'pad' | 'mouse') {
    if (next === inputMode) return;
    inputMode = next;
    if (next === 'pad') home = toPadMode(home);
    else mouseTravel = 0;
  }

  function handleInput(ev: InputEvent) {
    // B-22: the core owns the held set; keys only reach the UI, so they are reported to it.
    reportKey(ev);
    // D-040 / B-18 rev. am: going back to the game, or closing it: input locked.
    if (launching === 'returning' && ev.kind === 'down' && toAction(ev) === 'back') {
      lockHow = 'escaped (B)';
      core?.log('info', `${describeInput(ev, 'back', 'the return was cancelled (B): Home, the game stays paused')}`);
      returnHome('the return was cancelled (B)');
      return;
    }
    if (launching === 'returning' || launching === 'closing') {
      if (ev.kind === 'down') core?.log('info', `${describeInput(ev, null, `discarded (${launching === 'returning' ? 'going back to the game' : 'closing the game'}: input lock)`)}`);
      return;
    }
    if (ev.kind === 'up') {
      // D-019: a released direction may bump again on its next hold.
      const released = toAction({ ...ev, kind: 'down' });
      if (released) bumpedHolds[released] = false;
      return;
    }
    // A9: the input lock. During the black hold, Boot Up and Boot Down nothing in AIONE is
    // reachable; input is discarded (never queued). Its exceptions (alpha.9 QA-4):
    //   - the Boot Up's skip (A, B, Enter, Esc, a click);
    //   - lifecycle commands: Ctrl+Shift+R (restart; logged "restart (allowed during the input
    //     lock)") and Alt+F4 / the window's close (the Boot Down), handled before this point.
    if (phase !== 'home') {
      const action = toAction(ev);
      if (phase === 'boot' && ev.kind === 'down' && (action === 'confirm' || action === 'back') && presenter?.running) {
        core?.log('info', `${describeInput(ev, action, 'Boot Up skipped')}`);
        logSkip();
        presenter.skip();
      } else {
        core?.log('info', `${describeInput(ev, null, `discarded (${phase === 'bootdown' ? 'Boot Down' : phase === 'boot' ? 'Boot Up' : 'starting'}: input lock)`)}`);
      }
      return;
    }
    // D-046 (B-22 rev. ao): the launcher wait has no B (its 5 s limit is the way out).
    // B-22 (D-042): B leaves a normal launch after about 3 s ("B Back to Home" shown).
    if (launching === 'starting' && ev.kind === 'down' && toAction(ev) === 'back' && !launchBackOk) {
      // D-046 rev. ar: a window of the game, its launcher or Steam is up: B is neither shown nor acted on.
      core?.log('info', `${describeInput(ev, 'back', `B ignored (${showingNow ?? (followingId ? 'the launcher wait' : 'the start has just begun')})`)}`);
      return;
    }
    if (launching === 'starting' && !followingId && launchBackOk && ev.kind === 'down' && toAction(ev) === 'back') {
      lockHow = 'escaped (B)';
      const id = pendingId ?? runningId;
      core?.log('info', `${describeInput(ev, 'back', 'the start was cancelled (B): Home; a game appearing anyway is closed')}`);
      if (id) void core?.cancelStart(id).catch(() => {});
      pendingId = null;
      runningId = null;
      returnHome('the start was cancelled (B)');
      games = sortForHome(games);
      return;
    }
    // T1.3: input during the launch transition is discarded (never queued).
    if (launching === 'starting') {
      core?.log('info', `${describeInput(ev, null, 'discarded (launching a game: input lock)')}`);
      return;
    }
    // AIONE took input while a game runs: it is in front again (Guide, Alt-Tab).
    if (launching === 'ingame') returnHome('input while the game runs: AIONE is in front');
    perf.input(ev.source);
    sounds.resume();
    // D-008: how long the press waited between the core reading it and the UI acting on it.
    const delay = ev.atMs === undefined ? null : Math.max(0, Date.now() - ev.atMs);
    if (delay !== null) perf.inputDelay(delay);
    const waited = delay === null ? '' : `, core → UI ${delay} ms`;
    if (ev.source !== 'mouse') {
      // B-10 (rev. k): in mouse mode, only inputs that act on Home bring the ring back;
      // modifiers alone (the Alt of Alt-Tab or Alt+F4) and other keys change nothing.
      if (inputMode === 'mouse' && !countsAsHomeInput(ev)) {
        core?.log('info', `${describeInput(ev, null)} (mouse mode kept)${waited}`);
        return;
      }
      device = deviceAfter(device, ev.source);
      // B-10: the first pad or key press after mouse use only brings the ring back.
      if (inputMode === 'mouse') {
        setMode('pad');
        const where = dialogOpen
          ? 'in the dialog'
          : loadState === 'loaded'
            ? home.zone === 'drawer'
              ? `on drawer button "${DRAWER[home.drawer]?.label}"`
              : `on tile ${home.focus + 1}`
            : 'on Home';
        core?.log('info', `input ${ev.kind} ${ev.control} (${inputWhere(ev)}) -> ring back ${where} (after mouse), no move, no sound${waited}`);
        core?.log('info', `ring back ${where} (after mouse)`);
        perf.action(`ring back ${where}`);
        return;
      }
    }
    const action = toAction(ev);
    const outcome = action ? act(action, ev.kind === 'repeat' ? 'repeat' : 'press') : undefined;
    perf.action(`${ev.kind} ${ev.control} → ${action ?? 'no action'}${loadState === 'loaded' && !dialogOpen ? (home.zone === 'drawer' ? `, drawer ${DRAWER[home.drawer]?.label}` : `, ${where(home.focus)} "${games[home.focus]?.name ?? ''}"`) : ''}`);
    core?.log('info', describeInput(ev, action, `${outcome ?? ''}${waited}`.replace(/^, /, '')));
  }

  /** One action to the active context. Returns what it did, for the log line. */
  function act(action: UiAction, kind: 'press' | 'repeat'): string {
    // B-18: the close question takes the input while it is open.
    if (confirm) {
      if (confirm.single) {
        if (action === 'confirm' || action === 'back') return answer(false, action === 'confirm' ? 'A' : 'B');
        // B-29: one button: every direction is an edge.
        if (action === 'left' || action === 'right' || action === 'up' || action === 'down') return edge(action, kind, 'message: one button', 'dialog');
        return 'message: no action';
      }
      if (action === 'up' || action === 'down') return edge(action, kind, 'question: one row of buttons', 'dialog');
      if (action === 'left' || action === 'right') {
        const f = action === 'left' ? 'yes' : 'no';
        if (confirm.focus === f) return edge(action, kind, `question: the ${action === 'left' ? 'first' : 'last'} button`, 'dialog');
        confirm = { ...confirm, focus: f };
        return `question: focus ${f === 'yes' ? confirm.yes : 'Cancel'}, ${play('drawerMove')}`;
      }
      if (action === 'confirm') return answer(confirm.focus === 'yes', 'A');
      if (action === 'back') return answer(false, 'B');
      return 'question: no action';
    }
    if (dialogOpen) {
      // B-29: the copy question's buttons bump at their ends (it's a shared dialog now, rev. ba).
      if (flow.phase === 'ask') {
        if (action === 'up' || action === 'down') return edge(action, kind, 'copy question: one row of buttons', 'dialog');
        if (action === 'left' && flow.focus === BUTTON_COPY) return edge('left', kind, 'copy question: the first button', 'dialog');
        if (action === 'right' && flow.focus === BUTTON_SKIP) return edge('right', kind, 'copy question: the last button', 'dialog');
      }
      const before = flow;
      const [next, effect] = flowAction(flow, action);
      flow = next;
      applyEffect(effect);
      if (effect.type === 'decide') return `dialog decide, ${play('confirm')}`;
      if (effect.type === 'notNow') return `dialog not now, ${play('back')}`;
      if (next.phase === 'hidden') return `dialog closed, ${play(action === 'back' ? 'back' : 'confirm')}`;
      const moved = 'focus' in before && 'focus' in next && before.focus !== next.focus;
      return moved ? `dialog focus moved, ${play('rowMove')}` : 'dialog: nothing to do';
    }
    // B-26: the Quick Menu owns input while it is open (one interaction context).
    if (po !== null) return poAct(action, kind);
    if (qm) return qmAct(action, kind);
    if (action === 'menu') return openQm('a held Guide');
    if (action === 'bigLeft' || action === 'bigRight') return 'no action';
    if (loadState !== 'loaded') return 'home: no games';
    switch (action) {
      case 'left':
      case 'right': {
        const dir = action === 'left' ? -1 : 1;
        if (home.zone === 'drawer') {
          const [next, moved] = moveDrawer(home, dir);
          home = next;
          return moved ? `drawer ${DRAWER[home.drawer]?.label}, ${play('drawerMove')}` : edge(action, kind, 'end of drawer');
        }
        const [next, moved] = move(home, dir);
        if (moved) {
          // B-1 (rev. u): when the row scrolls, its low tick joins the click; rev. w: except on
          // the move landing on the first game or the last slot (the click alone).
          const scrolled = next.first !== home.first;
          const landing = next.focus === 0 || next.focus === next.slots - 1;
          home = next;
          holdRepeats = kind === 'repeat' ? holdRepeats + 1 : 0;
          const quiet = holdRepeats > QUIET_AFTER ? QUIET_DB : 0;
          return `${where(home.focus)}${scrolled ? ' (row scrolls)' : ''}, ${play(scrolled && !landing ? 'rowScroll' : 'rowMove', quiet)}`;
        }
        // B-14 (rev. w, final): at the row's end only a held repeat bumps (once per hold); EVERY
        // fresh press wraps, however fast.
        if (kind === 'repeat') return edge(action, kind, 'end of row');
        wrapping = true;
        if (wrapTimer) clearTimeout(wrapTimer);
        wrapTimer = setTimeout(() => {
          wrapping = false;
          wrapTimer = null;
        }, 260);
        home = wrapRow(home, dir);
        // D-021: the wrap is a fresh press: a hold continuing from it starts its repeats at full level.
        holdRepeats = 0;
        return `wrap to ${dir > 0 ? 'the first game' : 'the last slot'}: ${where(home.focus)}, ${play('wrap')}`;
      }
      case 'up':
      case 'down': {
        const [next, moved] = moveZone(home, action);
        home = next;
        if (!moved) return edge(action, kind, home.zone === 'drawer' ? 'nothing below the drawer' : 'nothing above the row');
        // B-1 (rev. r): entering a section plays that section's own move sound.
        return home.zone === 'drawer'
          ? `to the drawer: ${DRAWER[home.drawer]?.label}, ${play('drawerMove')}`
          : `back to the row: ${where(home.focus)}, ${play('rowMove')}`;
      }
      case 'confirm':
        if (home.zone === 'drawer') return openDrawer(home.drawer, 'press');
        // B-14: A does nothing on an empty slot (its "A OK" prompt is dimmed).
        if (home.focus >= games.length) return 'empty slot: A does nothing';
        return start(home.focus, 'press');
      case 'close': {
        // B-18: X (Delete) on the running game asks to close it.
        const g = home.zone === 'row' ? games[home.focus] : undefined;
        if (!g || g.id !== runningId) return 'no action (not the running game)';
        if (closingId === g.id) return `ignored ("${g.name}" is closing)`;
        confirm = { kind: 'close', id: g.id, title: `Close ${g.name}?`, text: 'Unsaved progress will be lost.', yes: 'Close', focus: 'no' };
        return `asked: close "${g.name}"?, ${play('optionsOpen')}`;
      }
      case 'guide':
        // B-7 (T1.3): Guide in front returns to the running game.
        if (runningId) return returnToGame(runningId, 'Guide');
        return 'no action (no game is running)';
      case 'options': {
        // B-14: "+ Options" on a game; the game's options come in T4. Rev. u: ≡ / Tab while the
        // Options toast shows closes it (like Switch's panel), with the close sound.
        if (optionsToast) return `options: ${closeOptionsToast('the Options button')}`;
        if (home.zone !== 'row' || home.focus >= games.length) return 'options: nothing here';
        if (toast) core?.log('info', 'toast replaced by the Options toast');
        toast = 'Coming in a later stage';
        optionsToast = true;
        if (toastTimer) clearTimeout(toastTimer);
        toastTimer = setTimeout(() => closeOptionsToast('timeout'), 2400);
        core?.log('info', `options for "${games[home.focus]?.name ?? ''}": they come in T4; toast "Coming in a later stage"`);
        return `options: toast, ${play('optionsOpen')}`;
      }
      case 'back':
        if (optionsToast) return `options: ${closeOptionsToast('B')}`;
        // B-14 (rev. r): B in the drawer returns to the game I was on, with the row's click.
        if (home.zone === 'drawer') {
          home = { ...home, zone: 'row' };
          return `back to the row: ${where(home.focus)}, ${play('rowMove')}`;
        }
        return 'home: nothing to go back to';
    }
  }

  /** B-1 (rev. s): the Options toast stands in for T4's Options panel: open / close sounds. */
  let optionsToast = false;
  function closeOptionsToast(how: string): string {
    if (!optionsToast) return 'nothing to close';
    optionsToast = false;
    toast = null;
    if (toastTimer) clearTimeout(toastTimer);
    toastTimer = null;
    // Rev. u: closing by a button plays the close sound; the timeout closes silently.
    if (how === 'timeout') {
      core?.log('info', 'options toast closed (timeout), no sound');
      return 'closed (timeout)';
    }
    return `closed (${how}), ${play('optionsClose')}`;
  }

  /** T1.2: the drawer's screens come in later stages (T2–T4); a press shows a plain toast. */
  let drawerPressed = $state<number | null>(null);
  let toast = $state<string | null>(null);
  let toastTimer: ReturnType<typeof setTimeout> | null = null;
  let drawerPressTimer: ReturnType<typeof setTimeout> | null = null;
  function openDrawer(index: number, how: string): string {
    const item = DRAWER[index];
    if (!item) return 'no such drawer button';
    drawerPressed = index;
    if (drawerPressTimer) clearTimeout(drawerPressTimer);
    drawerPressTimer = setTimeout(() => {
      drawerPressed = null;
      drawerPressTimer = null;
    }, 280);
    if (optionsToast) {
      optionsToast = false;
      core?.log('info', 'options toast replaced by the drawer toast (no close sound)');
    }
    // Rev. ax: the drawer's Power button opens Power options.
    if (item.label === 'Power') return `drawer Power (${how}): ${openPo('the drawer')}`;
    toast = 'Coming in a later stage';
    if (toastTimer) clearTimeout(toastTimer);
    toastTimer = setTimeout(() => {
      toast = null;
      toastTimer = null;
    }, 2400);
    core?.log('info', `drawer: ${item.label} (${how}): its screen comes in ${item.stage}; toast "Coming in a later stage"`);
    return `drawer ${item.label}: toast, ${play('error')}`;
  }

  function onDrawerHover(index: number | null) {
    if (inputMode !== 'mouse' || dialogOpen || phase !== 'home') return;
    home = hoverDrawer(home, index);
  }

  function onDrawerClick(index: number) {
    if (dialogOpen || phase !== 'home') return;
    setMode('mouse');
    home = { ...hoverDrawer(home, index), drawerHover: null };
    perf.action(`click drawer ${DRAWER[index]?.label}`);
    // A click activates at once (no game is started from the drawer); nothing stays highlighted.
    core?.log('info', `input click (mouse) -> ${openDrawer(index, 'click')}`);
  }

  /** Starting a game comes in T1.3: for now the press is shown, sounded and logged. */
  // ---------- T1.3: launching games ----------
  /** 'none'; 'starting' (the launch transition: Home still 0.55 s, then 0.2 s to black; input
   *  locked); 'ingame' (the game is in front; Home stays black until AIONE is back). */
  let launching = $state<'none' | 'starting' | 'ingame' | 'returning' | 'closing'>('none');
  /** D-040: the buttons and keys held right now (a press adds, a release removes). */
  /** B-22: a return in progress can be resumed anyway when its 3 s limit passes. */
  let returnResume: (() => void) | null = null;
  /** D-046 rev. ar: what the core says is showing during the start (null: nothing). */
  let showingNow: string | null = null;
  /** B-22 (D-042): B leaves a normal launch after about 3 s. */
  let launchBackOk = $state(false);
  /** B-22: why the current input-locking state ends (the core logs it). */
  let lockHow = 'done';
  /** The black veil over Home (its fade is 0.2 s, linear). */
  let veil = $state(false);
  /** B-18 (rev. am): the small progress ring on the "Closing <game>…" screen. */
  let closingSpinner = $state(false);
  /** D-040: the return's fade, the settle after the last release, and when to ask for a release. */
  const RETURN_FADE_MS = 300;
  const RELEASE_SETTLE_MS = 100;
  const RELEASE_HINT_MS = 2000;
  /** D-051: a start past its 45 s, watched quietly by the core (the tile shows "Starting…"). */
  let startingId = $state<string | null>(null);
  /** D-037: when the running game's window appeared (performance.now()). */
  let appearedAt = -1e9;
  /** D-032: the text on the veil while waiting for a launcher's game. */
  let veilText = $state<string | null>(null);
  /** B-18 (rev. ah): the game whose launcher wait B can cancel. */
  let followingId = $state<string | null>(null);
  /** The game that is running (started by AIONE), if any. */
  let runningId = $state<string | null>(null);
  let launchTimers: ReturnType<typeof setTimeout>[] = [];
  const LAUNCH_STILL_MS = 550;
  /** Rev. ab: the fade about 0.3 s, eased (measured on Switch). */
  const LAUNCH_FADE_MS = 300;
  /** D-026: black until the game's window appears, at most about 45 s. */
  const START_TIMEOUT_MS = Number(new URLSearchParams(location.search).get('starttimeout') ?? 45000);
  /** The game being started (until its window appears). */
  let pendingId: string | null = null;
  /** performance.now() when the launch's fade ends (a window that appears earlier waits for it). */
  let fadeEndAt = 0;
  /** D-030: the game being closed (X and A on it locked until it ends or 10 s pass). */
  let closingId = $state<string | null>(null);
  /** B-18: the close question (switching games, or X Close Software). */
  let confirm = $state<{ kind: 'switch' | 'close' | 'front' | 'power-restart' | 'power-shutdown' | 'message'; id: string; nextId?: string; title: string; text: string; yes: string; focus: 'yes' | 'no'; single?: boolean } | null>(null);
  /** L-13 / L-18 (T2.2): a message is the same dialog with one button (OK). */
  function message(title: string, text: string) {
    confirm = { kind: 'message', id: '', title, text, yes: 'OK', focus: 'no', single: true };
    core?.log('info', `message: "${title}"${text ? `: ${text}` : ''}`);
  }

  // ---------- B-26 (rev. ax): the Quick Menu and Power options (one interaction context each) ----------
  let qm = $state<{ row: number } | null>(null);
  /** Rev. ax: Power options (a short centered list), its focused item; null when closed. */
  let po = $state<number | null>(null);
  /** AIONE's own volume and mute, as the Quick Menu shows and changes them (B-24's store). */
  let uiVolume = $state(100);
  let uiMuted = $state(false);
  /** Rev. ax: the display's brightness (the core probes how; "none": the row says so). */
  let bright = $state<{ method: string; value: number | null; reason: string | null }>({ method: 'none', value: null, reason: 'Checking the display…' });
  let volumeSave: ReturnType<typeof setTimeout> | null = null;
  function openQm(why: string): string {
    if (qm) return 'Quick Menu: already open';
    qm = { row: 0 };
    void core?.brightnessGet().then((b) => (bright = b));
    return `Quick Menu opened (${why}), ${play('optionsOpen')}`;
  }
  function closeQm(how: string): string {
    qm = null;
    return `Quick Menu closed (${how}), ${play('optionsClose')}`;
  }
  function openPo(from: string): string {
    qm = null;
    po = 0;
    return `Power options opened (${from}), ${play('optionsOpen')}`;
  }
  function setVolume(volume: number, muted: boolean): string {
    uiVolume = Math.max(0, Math.min(100, Math.round(volume)));
    uiMuted = muted;
    sounds.setLevels(uiVolume / 100, uiMuted, sounds.levels().menu);
    BootPresenter.setLevel(uiVolume / 100, uiMuted);
    if (volumeSave) clearTimeout(volumeSave);
    volumeSave = setTimeout(() => void core?.changeSettings({ volume: uiVolume, muted: uiMuted }), 300);
    return `Quick Menu: AIONE volume ${uiVolume} %, ${uiMuted ? 'muted' : 'unmuted'} (AIONE's own; Windows' untouched)`;
  }
  function setBrightness(value: number): string {
    if (bright.method === 'none' || bright.value === null) return `Quick Menu: ${bright.reason ?? "brightness can't be changed"}`;
    const v = Math.max(0, Math.min(100, Math.round(value)));
    bright = { ...bright, value: v };
    void core?.brightnessSet(v);
    return `Quick Menu: brightness ${v} % (${bright.method})`;
  }
  function powerItem(i: number, how: string): string {
    const item = POWER_ITEMS[i];
    switch (i) {
      case 0: // Sleep Mode (T2.3): its Boot Down, then Windows sleeps; AIONE stays
        po = null;
        if (core) void bootDown(core, 'sleep');
        return `Power: ${item} (${how})`;
      case 1:
      case 2: {
        po = null;
        const restart = i === 1;
        confirm = {
          kind: restart ? 'power-restart' : 'power-shutdown',
          id: '',
          title: restart ? 'Restart the PC?' : 'Turn off the PC?',
          text: 'Open programs, including games, may lose unsaved progress.',
          yes: restart ? 'Restart' : 'Turn Off',
          focus: 'no',
        };
        core?.log('info', `asked: ${restart ? 'restart' : 'turn off'} the PC?`);
        return `Power: ${item} (${how}), ${play('confirm')}`;
      }
      case 3: // Restart AIONE: the Boot Down, then a restart
        po = null;
        if (core) void bootDown(core, 'restart');
        return `Power: ${item} (${how})`;
      case 4: // Close AIONE: the Boot Down, then exit
        po = null;
        if (core) void bootDown(core, 'close');
        return `Power: ${item} (${how})`;
    }
    return 'Power: nothing';
  }
  function poAct(action: UiAction, kind: 'press' | 'repeat' = 'press'): string {
    const f = po;
    if (f === null) return 'no Power options';
    if (action === 'up' || action === 'down') {
      const next = f + (action === 'down' ? 1 : -1);
      if (next < 0 || next >= POWER_ITEMS.length) return edge(action, kind, `Power options: the ${next < 0 ? 'first' : 'last'} item`, 'po');
      po = next;
      return `Power options: ${POWER_ITEMS[next]}, ${play('rowMove')}`;
    }
    if (action === 'confirm') return powerItem(f, 'A');
    if (action === 'back' || action === 'guide') {
      po = null;
      return `Power options closed (${action === 'back' ? 'B' : 'Guide'}), ${play('optionsClose')}`;
    }
    return 'Power options: no action';
  }
  // ---------- B-26 rev. ay: smooth sliders ----------
  /** A press glides the knob (about 120 ms); while a direction is held it is driven frame by frame. */
  let glide = $state(true);
  let drive: { row: string; dir: number; last: number; t: number; v: number; raf: number } | null = null;
  function stopDrive() {
    if (!drive) return;
    cancelAnimationFrame(drive.raf);
    core?.log('info', `Quick Menu: ${drive.row} held: ${Math.round(drive.v)} %`);
    drive = null;
    glide = true;
  }
  /** A held direction: a steady 50 % per second, applied live, never in visible jumps. */
  function driveSlider(row: string, dir: number): string {
    const now = performance.now();
    const at = row === 'Volume' ? uiVolume : (bright.value ?? 0);
    if ((dir < 0 && at === 0) || (dir > 0 && at === 100)) {
      stopDrive();
      return edge(dir < 0 ? 'left' : 'right', 'repeat', `Quick Menu: ${row} at its end`, row === 'Volume' ? 'slider-volume' : 'slider-brightness');
    }
    if (drive && drive.row === row && drive.dir === dir) {
      drive.last = now;
      return `Quick Menu: ${row} moving`;
    }
    stopDrive();
    const start = row === 'Volume' ? uiVolume : (bright.value ?? 0);
    drive = { row, dir, last: now, t: now, v: start, raf: 0 };
    glide = false;
    const tick = (t: number) => {
      const d = drive;
      if (!d) return;
      if (t - d.last > 220 || !qm || QM_ROWS[qm.row] !== d.row) return stopDrive();
      d.v = Math.max(0, Math.min(100, d.v + (d.dir * 50 * (t - d.t)) / 1000));
      if (d.v === 0 || d.v === 100) {
        // B-29: driven into an end: the knob bumps once for this hold.
        const end = Math.round(d.v);
        if (d.row === 'Volume') setVolume(end, uiMuted);
        else setBrightness(end);
        core?.log('info', edge(d.dir < 0 ? 'left' : 'right', 'press', `Quick Menu: ${d.row} held into its end`, d.row === 'Volume' ? 'slider-volume' : 'slider-brightness'));
        return stopDrive();
      }
      d.t = t;
      const r = Math.round(d.v);
      if (d.row === 'Volume' && r !== uiVolume) setVolume(r, uiMuted);
      if (d.row === 'Brightness' && r !== bright.value) setBrightness(r);
      d.raf = requestAnimationFrame(tick);
    };
    drive.raf = requestAnimationFrame(tick);
    return `Quick Menu: ${row} held: moving`;
  }
  /** A press: one step (left / right 1 %, LT / RT 10 %), glided. */
  function sliderStep(action: UiAction): number {
    stopDrive();
    glide = true;
    if (action === 'bigLeft') return -10;
    if (action === 'bigRight') return 10;
    return action === 'right' ? 1 : action === 'left' ? -1 : 0;
  }
  function qmAct(action: UiAction, kind: 'press' | 'repeat'): string {
    const q = qm;
    if (!q) return 'no Quick Menu';
    if (action === 'guide') return closeQm('Guide');
    if (action === 'menu') return 'Quick Menu: already open';
    if (action === 'up' || action === 'down') {
      const next = q.row + (action === 'down' ? 1 : -1);
      if (next < 0 || next >= QM_ROWS.length) return edge(action, kind, `Quick Menu: the ${next < 0 ? 'first' : 'last'} row`, 'qm');
      qm = { row: next };
      return `Quick Menu: ${QM_ROWS[next]}, ${play('rowMove')}`;
    }
    if (action === 'back') return closeQm('B');
    const slider = action === 'left' || action === 'right' || action === 'bigLeft' || action === 'bigRight';
    switch (QM_ROWS[q.row]) {
      case 'Power':
        return action === 'confirm' ? openPo('the Quick Menu') : 'Quick Menu: no action';
      case 'Brightness':
        if (slider && kind === 'repeat' && (action === 'left' || action === 'right')) return driveSlider('Brightness', action === 'right' ? 1 : -1);
        if (slider) {
          const d = sliderStep(action);
          if (bright.value !== null && ((d < 0 && bright.value === 0) || (d > 0 && bright.value === 100))) return edge(d < 0 ? 'left' : 'right', kind, 'Quick Menu: brightness at its end', 'slider-brightness');
          return setBrightness((bright.value ?? 0) + d);
        }
        return 'Quick Menu: no action';
      case 'Volume':
        if (slider && kind === 'repeat' && (action === 'left' || action === 'right')) return driveSlider('Volume', action === 'right' ? 1 : -1);
        if (slider) {
          const d = sliderStep(action);
          if ((d < 0 && uiVolume === 0) || (d > 0 && uiVolume === 100)) return edge(d < 0 ? 'left' : 'right', kind, 'Quick Menu: volume at its end', 'slider-volume');
          return setVolume(uiVolume + d, uiMuted);
        }
        if (action === 'confirm') return setVolume(uiVolume, !uiMuted);
        return 'Quick Menu: no action';
      default:
        if (action === 'confirm' || action === 'left' || action === 'right') {
          showMessage('Coming in a later stage');
          return `Quick Menu: Theme: coming in a later stage, ${play('error')}`;
        }
        return 'Quick Menu: no action';
    }
  }

  /** B-11: the row's order: the running game first (B-18), then played games most recent first,
   *  then the never-played ones A-Z. */
  function sortForHome(list: GameView[]): GameView[] {
    const when = (g: GameView) => (g.lastPlayed ? Date.parse(g.lastPlayed) : NaN);
    return [...list].sort((a, b) => {
      if (runningId && (a.id === runningId) !== (b.id === runningId)) return a.id === runningId ? -1 : 1;
      const x = when(a), y = when(b);
      if (!isNaN(x) && !isNaN(y)) return y - x || a.id.localeCompare(b.id);
      if (!isNaN(x)) return -1;
      if (!isNaN(y)) return 1;
      return a.name.toLowerCase().localeCompare(b.name.toLowerCase()) || a.id.localeCompare(b.id);
    });
  }

  /** The focus on a game by id (after the row's order changed), keeping it in view. */
  function focusGame(id: string) {
    const i = games.findIndex((g) => g.id === id);
    if (i < 0) return;
    home = { ...home, zone: 'row', focus: i, first: Math.max(0, Math.min(i, home.first, Math.max(0, home.slots - 4))) };
    if (i < home.first || i > home.first + 3) home = { ...home, first: Math.max(0, i - 3) };
  }

  function start(index: number, how: string): string {
    const g = games[index];
    if (!g) return 'nothing to start';
    if (launching !== 'none') return 'ignored (a launch is in progress)';
    // D-051: a start being watched quietly: A on it says so; another game ends the watch.
    if (startingId && games[index]?.id === startingId) {
      showMessage(`${games[index]?.name} is still starting`);
      return `"${games[index]?.name}" is still starting (watching quietly), ${play('error')}`;
    }
    if (startingId) {
      core?.log('info', `the quiet watch of "${games.find((x) => x.id === startingId)?.name ?? startingId}" ends (another game is started)`);
      startingId = null;
    }
    // D-030: while a game is closing, A on it (or on another game) waits.
    if (closingId) return `ignored ("${games.find((x) => x.id === closingId)?.name ?? closingId}" is closing)`;
    pressFeedback(index);
    // An already-running game is brought to the front, never started twice (Part B).
    if (runningId === g.id) return returnToGame(g.id, how);
    // B-18: one game at a time: another game while one runs asks first (Cancel focused).
    if (runningId) {
      const r = games.find((x) => x.id === runningId);
      confirm = {
        kind: 'switch',
        id: runningId,
        nextId: g.id,
        title: `Close ${r?.name ?? 'the running game'} and start ${g.name}?`,
        text: `Unsaved progress in ${r?.name ?? 'the running game'} will be lost.`,
        yes: 'Close and start',
        focus: 'no',
      };
      return `asked: close "${r?.name ?? runningId}" and start "${g.name}"?, ${play('optionsOpen')}`;
    }
    launching = 'starting';
    launchBackOk = false;
    // D-046 rev. ar: from 3 s, "B Cancel start" only while NOTHING is showing (no window of the
    // game, its launcher or Steam); the core says what is.
    const checkShowing = () => {
      if (launching !== 'starting' || followingId) return;
      void core?.startShowing(g.id).then((what) => {
        if (launching !== 'starting') return;
        showingNow = what;
        const elapsed = performance.now() - startedAt;
        launchBackOk = !what && !followingId && elapsed >= 3000;
        // B-27: Windows' permission prompt: say so on the black screen (answer it there).
        veilText = what?.startsWith('Windows is asking') ? what : elapsed >= 3000 || followingId ? `Starting ${g.name}…` : null;
        launchTimers.push(setTimeout(checkShowing, 500));
      });
    };
    const startedAt = performance.now();
    launchTimers.push(setTimeout(checkShowing, 500));
    const t0 = performance.now();
    fadeEndAt = t0 + LAUNCH_STILL_MS + LAUNCH_FADE_MS;
    const s = play('launch');
    launchTimers.push(setTimeout(() => (veil = true), LAUNCH_STILL_MS));
    const req = core ? core.launchGame(g.id) : Promise.reject(new Error('no core'));
    req
      .then((r) => {
        const ms = Math.round(performance.now() - t0);
        core?.log('info', r.alreadyRunning ? `launch "${r.name}": already running: brought to the front (${r.foreground ?? '?'})` : `launch "${r.name}": started (${r.how}${r.pid ? `, process ${r.pid}` : ''}), ${ms} ms after the press`);
        runningId = g.id;
        // B-11: the start is recorded by the core; the row shows it at once (the game first).
        games = sortForHome(games.map((x) => (x.id === g.id ? { ...x, lastPlayed: new Date().toISOString(), playCount: x.playCount + 1 } : x)));
        launchTimers.push(setTimeout(() => focusGame(g.id), Math.max(0, LAUNCH_STILL_MS + LAUNCH_FADE_MS - ms)));
        // D-026: black, input locked, until the game's window appears (or the timeout).
        if (r.alreadyRunning) {
          launching = 'ingame';
          return;
        }
        pendingId = g.id;
        launchTimers.push(setTimeout(() => {
          if (launching !== 'starting' || pendingId !== g.id) return;
          pendingId = null;
          launching = 'none';
          veil = false;
          // D-051 (option b): the start becomes a quiet watch (up to about 3 min, in the core):
          // the tile shows "Starting…"; it isn't the running game (Continue / Close never act).
          startingId = g.id;
          if (runningId === g.id) runningId = null;
          core?.log('warn', `launch "${g.name}": no window after ${Math.round(START_TIMEOUT_MS / 1000)} s: Home again ("Still starting"); watching quietly for a late window (up to about 3 min)`);
          showMessage(`Still starting ${g.name}…`);
        }, START_TIMEOUT_MS));
      })
      .catch((e: unknown) => {
        const reason = e instanceof Error ? e.message : String(e);
        const ms = Math.round(performance.now() - t0);
        core?.log('warn', `launch "${g.name}" failed: ${reason}`);
        // After the fade, back to Home with a plain message.
        launchTimers.push(setTimeout(() => {
          veil = false;
          launching = 'none';
          message(`Couldn't start ${g.name}`, reason);
          play('error');
        }, Math.max(0, LAUNCH_STILL_MS + LAUNCH_FADE_MS - ms)));
      });
    return `start "${g.name}" (${how}), ${s}`;
  }

  /** B-7: A or Guide on the running game brings it back to the front. */
  /**
   * D-040 (B-20 rev. am): back into the game: fade AIONE to black (0.3 s, eased) with input locked;
   * wait until every button and key is released (plus 0.1 s; "Release the buttons" after 2 s); only
   * then resume the game and hand it the front.
   */
  function returnToGame(id: string, how: string): string {
    const g = games.find((x) => x.id === id);
    const name = g?.name ?? id;
    launching = 'returning';
    veil = true;
    veilText = null;
    const t0 = performance.now();
    let freeSince = -1;
    const tick = () => {
      if (launching !== 'returning') return;
      const now = performance.now();
      if (now - t0 < RETURN_FADE_MS) return void launchTimers.push(setTimeout(tick, 50));
      void core?.heldNow().then((h) => {
        if (launching !== 'returning') return;
        const t = performance.now();
        if (h.length > 0) {
          freeSince = -1;
          if (t - t0 > RELEASE_HINT_MS) veilText = 'Release the buttons';
          launchTimers.push(setTimeout(tick, 50));
          return;
        }
        if (freeSince < 0) freeSince = t;
        if (t - freeSince < RELEASE_SETTLE_MS) return void launchTimers.push(setTimeout(tick, 50));
        resume(t);
      });
    };
    const resume = (now: number) => {
      if (launching !== 'returning') return;
      veilText = null;
      core?.log('info', `back to "${name}": waiting for the buttons to be released (${Math.round(now - t0)} ms)`);
      lockHow = 'done (released)';
      core
        ?.returnToGame(id)
        .then((m) => {
          // alpha.27 QA-3: the core logs the return once (`back to …: OK via …`).
          void m;
          launching = 'ingame';
        })
        .catch((e: unknown) => {
          const reason = e instanceof Error ? e.message : String(e);
          core?.log('warn', `back to "${name}" failed: ${reason}`);
          returnHome(`couldn't go back to "${name}"`);
          message(`Couldn't return to ${name}`, reason);
          play('error');
        });
    };
    returnResume = () => resume(performance.now());
    tick();
    return `back to the running game "${name}" (${how}): fading, then waiting for the buttons, ${play('confirm')}`;
  }

  /** B-18: the answer to the close question. */
  function answer(yes: boolean, how: string): string {
    const q = confirm;
    if (!q) return 'no question';
    confirm = null;
    if (q.kind === 'message') return `message: OK (${how}), ${play('back')}`;
    const running = games.find((x) => x.id === q.id);
    if (!yes) return `question: Cancel (${how}), ${play('back')}`;
    if (q.kind === 'power-restart' || q.kind === 'power-shutdown') {
      const action = q.kind === 'power-restart' ? 'restart' : 'shutdown';
      core?.powerPc(action).catch((e: unknown) => message("Couldn't do that", e instanceof Error ? e.message : String(e)));
      return `question: ${q.yes} (${how}): ${action} the PC, ${play('confirm')}`;
    }
    // B-27: Windows refused the close: bring the game to the front to close it there.
    if (q.kind === 'front') {
      const i = games.findIndex((x) => x.id === q.id);
      if (i >= 0) focusGame(q.id);
      return `question: ${q.yes} (${how}): ${returnToGame(q.id, how)}`;
    }
    // B-18 (rev. am): closing for sure. A "Closing <game>…" loading screen (black, a small progress
    // ring, input locked) until every process of the game has ended (the core asks it to close,
    // gives it about 3 s, then ends it); then Home, or the new game's launch.
    const name = running?.name ?? q.id;
    core?.log('info', `close "${name}": asked by the user (${q.kind === 'switch' ? 'to start another game' : 'Close Software'})`);
    closingId = q.id;
    launching = 'closing';
    veil = true;
    veilText = `Closing ${name}…`;
    closingSpinner = true;
    const finish = () => {
      closingSpinner = false;
      if (closingId === q.id) closingId = null;
      if (runningId === q.id) {
        runningId = null;
        games = sortForHome(games);
      }
    };
    core
      ?.closeGame(q.id)
      .then(() => {
        finish();
        core?.log('info', `close "${name}": done`);
        if (q.kind === 'switch' && q.nextId) {
          // D-031: the old game has ended; the new game starts now (from the black screen).
          launching = 'none';
          veilText = null;
          const next = games.findIndex((x) => x.id === q.nextId);
          if (next >= 0) core?.log('info', start(next, 'after closing the running game'));
          else returnHome(`"${name}" closed`);
        } else {
          returnHome(`"${name}" closed`);
        }
      })
      .catch((e: unknown) => {
        finish();
        const reason = e instanceof Error ? e.message : String(e);
        core?.log('warn', `close "${name}" failed: ${reason}`);
        returnHome(`closing "${name}" failed`);
        if (reason.includes('runs as administrator')) {
          // B-27: plainly, and a way to close it there (Cancel focused).
          confirm = { kind: 'front', id: q.id, title: `Bring ${name} to the front to close it there?`, text: reason, yes: 'Bring to front', focus: 'no' };
          core?.log('info', `asked: bring "${name}" to the front to close it there?`);
          return;
        }
        showMessage(reason);
        play('error');
      });
    return `question: ${q.yes} (${how}), ${play('confirm')}`;
  }

  /** AIONE is in front again (Guide, the game ended): Home fades in, the focus on that game. */
  function returnHome(why: string) {
    if (launching === 'none' && !veil) return;
    launchTimers.forEach(clearTimeout);
    launchTimers = [];
    launching = 'none';
    veil = false;
    veilText = null;
    followingId = null;
    if (runningId) focusGame(runningId);
    core?.log('info', `Home again: ${why}${runningId ? `; focus on the running game` : ''}`);
  }

  /** A plain message (a failed start): longer than a toast, the same place. */
  function showMessage(text: string, ms = 5000) {
    toast = text;
    if (toastTimer) clearTimeout(toastTimer);
    toastTimer = setTimeout(() => {
      toast = null;
      toastTimer = null;
    }, ms);
  }

  function pressFeedback(index: number) {
    pressed = index;
    if (pressTimer) clearTimeout(pressTimer);
    pressTimer = setTimeout(() => {
      pressed = null;
      pressTimer = null;
    }, extrasOn ? 200 : 90);
  }

  /** T2.3: AIONE asked Windows to sleep and waits (locked) for the wake. */
  let sleeping = false;
  /** T2.3: the resume transition: back to Home, its focus and any paused game as they were. */
  function wake(why: string) {
    if (!sleeping) return;
    sleeping = false;
    phase = 'home';
    perf.setScreen('Home');
    core?.log('info', `resume (${why}): back to Home; the focus and any paused game as they were`);
  }
  /** Rev. ba: `--show-copy-question`: the question as a preview; nothing decided, copied or recorded. */
  let previewCopy = false;
  function applyEffect(effect: FlowEffect) {
    if (previewCopy && (effect.type === 'decide' || effect.type === 'notNow')) {
      previewCopy = false;
      flow = { phase: 'hidden' };
      core?.log('info', `copy question (preview, --show-copy-question): ${effect.type === 'notNow' ? 'Not now' : effect.accept ? 'Copy' : "Don't copy"}: nothing copied or recorded`);
      return;
    }
    if (effect.type === 'decide') decide(effect.accept);
    if (effect.type === 'notNow') core?.log('info', 'copy offer: user chose Not now (nothing recorded; asked again at the next start)');
  }

  function decide(accept: boolean) {
    core?.log('info', `copy offer: user chose ${accept ? 'Copy' : "Don't copy"}`);
    core?.migrationDecide(accept).catch((err: unknown) => {
      core?.log('error', `migration_decide failed: ${String(err)}`);
    });
  }

  /** A click on a dialog button: logged with the action it activated (alpha.2 QA-5). */
  function onChoose(accept: boolean) {
    core?.log('info', `input click "${accept ? 'Copy' : "Don't copy"}" (mouse) -> action ${accept ? 'copy' : 'dont-copy'}, ${play('confirm')}`);
    const [next, effect] = choose(flow, accept);
    flow = next;
    applyEffect(effect);
  }

  function onHover(index: number | null) {
    if (inputMode !== 'mouse' || dialogOpen) return;
    home = hoverTile(home, index);
  }

  /** L-06: a click activates and leaves no hover behind; never starts a game with a single
   *  click on an unselected tile. */
  function onTileClick(index: number) {
    if (dialogOpen) return;
    setMode('mouse');
    perf.input('mouse');
    sounds.resume();
    const g = games[index];
    const [next, result] = clickTile(home, index);
    home = next;
    perf.action(`click tile ${index + 1} "${g?.name ?? ''}" (${result})`);
    if (result === 'select') {
      core?.log('info', `input click tile ${index + 1} "${g?.name ?? ''}" (mouse) -> select (first click), ${play('rowMove')}`);
    } else {
      core?.log('info', `input click tile ${index + 1} "${g?.name ?? ''}" (mouse) -> ${start(index, 'second click')}`);
    }
  }

  function onKey(e: KeyboardEvent) {
    if (isBlockedBrowserKey(e) || e.key === 'Tab' || e.key === ' ') e.preventDefault();
    if (!firstKeyLogged && e.type === 'keydown') {
      firstKeyLogged = true;
      core?.log('info', `first keyboard event ${Math.round(performance.now())} ms after the page started (page focused: ${document.hasFocus()})`);
    }
    if (e.type === 'keydown' && isRestartShortcut(e) && !e.repeat) {
      e.preventDefault();
      // A9: lifecycle commands are the input lock's exceptions (Ctrl+Shift+R, Alt+F4).
      core?.log('info', phase === 'home' ? 'restart requested (Ctrl+Shift+R)' : `Ctrl+Shift+R: restart (allowed during the input lock, ${phase})`);
      core?.restart().catch((err: unknown) => core?.log('error', `restart failed: ${String(err)}`));
      return;
    }
    if (e.type === 'keyup') keys.keyup(e);
    else keys.keydown(e);
  }

  function onMouseMove(e: MouseEvent) {
    // movementX / Y are 0 for the synthetic move Windows sends when a window appears under a
    // resting pointer, so only real movement counts.
    mouseTravel += Math.abs(e.movementX) + Math.abs(e.movementY);
    if (inputMode !== 'mouse' && mouseTravel > MOUSE_THRESHOLD_PX) {
      setMode('mouse');
      perf.input('mouse');
      // The tile under the pointer gets its hover now (mouseenter fired while in pad mode).
      const el = document.elementFromPoint(e.clientX, e.clientY)?.closest<HTMLElement>('[data-testid="tile"]');
      if (el && !dialogOpen) home = hoverTile(home, Number(el.dataset['index']));
    }
  }

  function onWheel(e: WheelEvent) {
    if (e.deltaY === 0 || dialogOpen) return;
    setMode('mouse');
    perf.input('mouse');
    [home] = wheel(home, e.deltaY > 0 ? 1 : -1);
  }

  function onResize() {
    home = withVisible(home, visibleTiles(window.innerWidth, window.innerHeight));
  }

  /** D-007: decoded pictures are kept here so the browser keeps them ready. */
  let decoded: HTMLImageElement[] = [];

  /** D-007: every tile and icon picture is loaded and decoded before Home takes input, so
   *  moving along the row never decodes a picture. Waits at most 5 s. */
  async function decodeAhead(list: GameView[]): Promise<void> {
    if (!core) return;
    const c = core;
    const started = performance.now();
    const jobs: Promise<string>[] = [];
    const keep: HTMLImageElement[] = [];
    for (const g of list) {
      {
        // The core's copy when it exists, else the original picture (D-010).
        const slot = g.tile ? 'tile' : g.tileArt;
        if (!slot) continue;
        const url = c.mediaUrl(g.id, slot, g.tile ?? undefined);
        if (!url) continue;
        const img = new Image();
        img.decoding = 'async';
        img.src = url;
        keep.push(img);
        jobs.push(
          img
            .decode()
            .then(() => {
              perf.picture(`${slot} "${g.name}" (decoded ahead)`, img.naturalWidth, img.naturalHeight);
              return `${img.naturalWidth}×${img.naturalHeight}`;
            })
            .catch(() => 'failed'),
        );
      }
    }
    // D-010: Home must be ready within 2 s; pictures not decoded by then decode on their own.
    const timeout = new Promise<'timeout'>((r) => setTimeout(() => r('timeout'), 1500));
    const done = await Promise.race([Promise.all(jobs), timeout]);
    decoded = keep;
    const ms = Math.round(performance.now() - started);
    if (done === 'timeout') {
      c.log('warn', `tile pictures: decoding ahead took over 1.5 s; Home starts anyway`);
    } else {
      const sizes = [...new Set(done)].join(', ') || 'none';
      const failed = done.filter((d) => d === 'failed').length;
      c.log(
        'info',
        `tile pictures ready: ${done.length - failed} decoded ahead in ${ms} ms (sizes ${sizes}; ${decoded.length} kept ready)${failed ? `, ${failed} failed` : ''}`,
      );
    }
  }

  async function setLibrary(l: LibraryLoad) {
    const shown = l.games.filter((g) => !g.hidden);
    await decodeAhead(shown);
    // Recorded only now, so "loaded" means Home is really ready to take input.
    library = l;
    games = shown;
    loadState = l.state === 'error' ? 'error' : games.length > 0 ? 'loaded' : 'empty';
    home = withCount(home, games.length);
    core?.log('info', `home shows ${games.length} games (${l.games.length - games.length} hidden)`);
  }

  async function reloadLibrary(c: Core) {
    try {
      await setLibrary(await c.libraryLoad());
    } catch (err) {
      loadState = 'error';
      c.log('error', `library_load failed: ${String(err)}`);
    }
  }

  $effect(() => {
    if (phase === 'home') perf.setScreen(dialogOpen ? 'home + copy dialog' : 'home');
  });

  function bootPresenter(c: Core): BootPresenter | null {
    if (!bootHost) return null;
    presenter ??= new BootPresenter(bootHost, (level, line) => c.log(level, line));
    return presenter;
  }

  /** D-010: a copy made in the background is decoded first, then swapped in. QA-2: copies that
   *  arrive before Home (during the black hold or the Boot Up) wait, so nothing is decoded while
   *  the boot animation plays. */
  const pendingTiles: { id: string; tile: string }[] = [];
  function swapTile(c: Core, { id, tile }: { id: string; tile: string }) {
    const url = c.mediaUrl(id, 'tile', tile);
    const img = new Image();
    img.src = url;
    img
      .decode()
      .then(() => {
        decoded = [...decoded, img];
        perf.picture(`tile "${id}" copy (decoded before the swap)`, img.naturalWidth, img.naturalHeight);
        games = games.map((g) => (g.id === id ? { ...g, tile } : g));
      })
      .catch((err: unknown) => c.log('warn', `tile copy for "${id}" could not be decoded: ${String(err)}`));
  }

  /** D-014 (rev. r): when the skip came, and whether Home was ready then. */
  function logSkip() {
    const at = ((performance.now() - bootStartedAt) / 1000).toFixed(2);
    core?.log(
      'info',
      homeReadyAt
        ? `skip at ${at} s into the Boot Up; Home had been ready for ${((performance.now() - homeReadyAt) / 1000).toFixed(2)} s`
        : `skip at ${at} s into the Boot Up; Home was not ready yet`,
    );
  }

  /** D-014: mount Home under the black layer (Window B: when its fade to black begins). */
  function prepareHome() {
    if (homeEarly) return;
    homeEarly = true;
    core?.log('info', 'Home prepared under the black screen (its first frames drawn before it shows)');
  }

  /** Two animation frames: Home's first frames are drawn (still covered) before it shows. */
  const twoFrames = () => new Promise<void>((r) => requestAnimationFrame(() => requestAnimationFrame(() => r())));

  /** A9: one Boot Up presentation, decided once: the custom video, or the default Window B
   *  (and once only, the default as the controlled fallback when the custom one cannot start). */
  async function bootUp(c: Core) {
    const m = bootMedia;
    const p = bootPresenter(c);
    if (!m || !p) return;
    if (!m.settings.showSplash) {
      c.log('info', 'Boot Up: none (the Boot Up is switched off in the settings)');
      return;
    }
    phase = 'boot';
    bootStartedAt = performance.now();
    perf.setScreen('Boot Up');
    perf.action('Boot Up');
    let outcome: Outcome;
    if (m.up && !info?.bootDefault) {
      c.log('info', `Boot Up: custom video (${m.up.source}, AIONE 4's copy), sound ${m.settings.splashAudio ? 'on' : 'off'}${m.settings.playToEnd ? ', to the end' : `, ${m.settings.durationMs} ms`}`);
      outcome = await p.playVideo('up', c.bootUrl('up', m.up.key), {
        muted: !m.settings.splashAudio,
        capMs: m.settings.playToEnd ? undefined : m.settings.durationMs,
      });
      if (outcome.startsWith('Failed') && !outcome.includes('mid-playback') && phase === 'boot') {
        c.log('warn', `Boot Up: the custom video failed (${outcome.slice(8)}); the default Window B plays instead (the one fallback)`);
        outcome = await p.playDefault('up', bootUpSound, prepareHome);
      }
    } else {
      c.log('info', `Boot Up: default Window B (${info?.bootDefault ? '--boot=default for this run' : 'no custom video'})`);
      outcome = await p.playDefault('up', bootUpSound, prepareHome);
    }
    c.log('info', `Boot Up outcome: ${outcome}`);
  }

  /** A9 (Close AIONE): the Boot Down plays, then AIONE closes; the screen stays black until the
   *  Boot Down sound ends. Starts once, from any phase. */
  async function bootDown(c: Core, then: 'close' | 'restart' | 'sleep' = 'close') {
    if (phase === 'bootdown') return;
    phase = 'bootdown';
    perf.setScreen('Boot Down');
    perf.action('Boot Down');
    c.log('info', `${then === 'restart' ? 'Restart AIONE' : then === 'sleep' ? 'Sleep Mode' : 'Close AIONE'}: the Boot Down starts (input locked)`);
    const p = bootPresenter(c);
    p?.stop();
    const m = bootMedia;
    let outcome: Outcome = 'Completed';
    if (p) {
      if (m?.settings.showSplash !== false && m?.down && !info?.bootDefault) {
        c.log('info', `Boot Down: custom video (${m.down.source}, AIONE 4's copy)`);
        outcome = await p.playVideo('down', c.bootUrl('down', m.down.key), { muted: !m.settings.splashAudio });
        if (outcome.startsWith('Failed') && !outcome.includes('mid-playback')) {
          c.log('warn', `Boot Down: the custom video failed (${outcome.slice(8)}); the default Window B plays instead`);
          outcome = await p.playDefault('down', bootDownSound);
        }
      } else {
        c.log('info', `Boot Down: default Window B (${info?.bootDefault ? '--boot=default for this run' : 'no custom video'})`);
        outcome = await p.playDefault('down', bootDownSound);
      }
    }
    if (then === 'sleep') {
      // T2.3: Windows sleeps; AIONE stays, locked in its Boot Down state until it wakes.
      sleeping = true;
      c.log('info', `Sleep Mode: Boot Down ${outcome}; Windows sleeps`);
      await c.sleepPc().catch((e: unknown) => {
        c.log('warn', `Sleep Mode: ${String(e)}; back to Home`);
        wake('sleep failed');
      });
      return;
    }
    if (then === 'restart') {
      c.log('info', `Restart AIONE: Boot Down ${outcome}; restarting`);
      await c.restart();
      return;
    }
    await c.closeNow(`Boot Down ${outcome}`);
  }

  onMount(() => {
    const unlisteners: Unlisten[] = [];
    let disposed = false;

    (async () => {
      const c = await loadCore();
      if (disposed) return;
      core = c;
      unlisteners.push(
        await c.onPad((ev) => {
          if (ev.kind === 'connected' || ev.kind === 'disconnected') {
            pads = applyPadConnection(pads, ev.pad + 1, ev.kind === 'connected', ev.seq);
            return;
          }
          const input = fromPadEvent(ev);
          if (input) handleInput(input);
        }),
        await c.onCopyProgress((p) => (flow = flowProgress(flow, p))),
        await c.onCopyDone((r) => {
          flow = flowDone(flow, r);
          void reloadLibrary(c);
        }),
        await c.onStatus((s) => (status = s)),
        // D-010: a copy made in the background is decoded first, then swapped in.
        await c.onTile((t) => (phase === 'home' ? swapTile(c, t) : pendingTiles.push(t))),
      );
      const slots = await c.padSlots();
      pads = applyPadSlots(
        pads,
        slots.slots.map((s) => s + 1),
        slots.seq,
      );
      c.log('info', `controller slots at start: [${slots.slots.map((s) => s + 1).join(', ')}] (as of seq ${slots.seq})`);
      info = await c.appInfo();
      document.documentElement.style.setProperty('--scale', String(info.interfaceScale / 100));
      if (params.get('preset') === 'dark') document.documentElement.dataset['preset'] = 'dark';
      unlisteners.push(await c.onCloseRequested(() => void bootDown(c)));
      // T1.3: the running game ended: the core brought AIONE to the front; Home, focus on it.
      unlisteners.push(
        await c.onGameExited((e) => {
          const id = e.id;
          if (pendingId === id) pendingId = null;
          if (startingId === id) startingId = null; // D-051: the quiet watch ended
          // D-031: an end that arrives after a "Close and start" already cleared the game (and
          // maybe started the next one) is only logged: no return Home, no change to the launch.
          if (runningId !== id) {
            core?.log('info', `"${e.name}" ended (already closed by AIONE)`);
            games = sortForHome(games);
            return;
          }
          returnHome(`"${e.name}" ended`);
          runningId = null;
          games = sortForHome(games);
          focusGame(id);
        }),
      );
      // B-27: Windows' refusals as a plain message.
      // B-27's refusals as a message; D-047's hint (from the hook) as a plain toast.
      unlisteners.push(await c.onResumed((e) => wake(`Windows woke after about ${e.asleepS} s`)));
      unlisteners.push(await c.onNotice((e) => (e.text.startsWith('Another program') ? showMessage(e.text, 9000) : message("AIONE can't do that", e.text))));
      // D-032: the launcher ended soon after the launch: black again, "Starting <game>…", input
      // locked, until the real game's window (the launch completes) or the end (Home).
      unlisteners.push(
        await c.onGameFollowing((e) => {
          if (runningId !== e.id && pendingId !== e.id) return;
          launchTimers.forEach(clearTimeout);
          launchTimers = [];
          pendingId = e.id;
          launching = 'starting';
          veil = true;
          veilText = `Starting ${e.name}…`;
          followingId = e.id;
          fadeEndAt = performance.now();
          core?.log('info', `"${e.name}": its launcher ended; waiting for the game (black, input locked)`);
        }),
      );
      // B-18 (rev. ae): the close's hint, as the core decided it (a window to answer in, or none).
      unlisteners.push(
        await c.onCloseHint((e) => {
          if (closingId === e.id) showMessage(e.text, 10000);
        }),
      );
      // B-22: a state reached its limit (the core decides): per the table.
      unlisteners.push(
        await c.onLockExpired((e) => {
          lockSent = 'none';
          if (e.kind === 'going back to the game' && launching === 'returning') {
            core?.log('info', 'the return waited 3 s for the buttons: resuming anyway');
            returnResume?.();
          } else if (e.kind === 'launching a game' && launching === 'starting') {
            showMessage(`Still starting ${games.find((x) => x.id === pendingId)?.name ?? 'the game'}…`);
            returnHome('the launch reached its limit');
          } else if (launching !== 'none' && launching !== 'ingame' && e.kind !== 'closing the game') {
            returnHome(`${e.kind} reached its limit`);
          }
        }),
      );
      // D-026: the game's window appeared: only now is AIONE behind the game ('ingame').
      unlisteners.push(
        await c.onGameWindow((e) => {
          // D-051: a late window of a start being watched quietly: followed as the running game.
          if (startingId === e.id && launching === 'none') {
            startingId = null;
            runningId = e.id;
            veil = true;
            launching = 'ingame';
            core?.log('info', `"${e.name}": its window appeared late (after the 45 s); followed`);
            return;
          }
          if (launching === 'starting' && pendingId === e.id) {
            pendingId = null;
            veilText = null;
            followingId = null;
            appearedAt = performance.now();
            core?.log('info', `"${e.name}": its window appeared after ${e.ms} ms; the launch is complete`);
            // The transition always completes: a window that appears early waits for the fade.
            launchTimers.push(setTimeout(() => {
              if (launching === 'starting') launching = 'ingame';
            }, Math.max(0, fadeEndAt - performance.now())));
          }
        }),
      );
      // T1.3 (B-7): AIONE came back to the front while a game runs (Guide, Alt-Tab).
      const onFocus = () => {
        if (launching !== 'ingame') return;
        // D-037: in the first seconds after the game appeared, AIONE getting the front without
        // Guide is the game swapping its window: hand the front back instead of showing Home.
        const id = runningId;
        if (id && performance.now() - appearedAt < 5000) {
          // alpha.29 QA-1: input is locked AT ONCE (the black launcher wait), while the core is
          // asked; Home is never live in between (it was for 39–119 ms).
          launching = 'starting';
          followingId = id;
          pendingId = id;
          veil = true;
          veilText = null;
          void core?.msSinceGuide().then((ms) => {
            if (launching !== 'starting' || followingId !== id) return;
            if (ms < 1500) return returnHome('AIONE is in front again');
            core
              ?.handBack(id)
              .then(() => {
                core?.log('info', `the game swapped its window: front handed back (${ms >= 999999 ? 'no Guide' : `last Guide ${ms} ms ago`})`);
                if (launching === 'starting' && followingId === id) {
                  followingId = null;
                  pendingId = null;
                  launching = 'ingame';
                }
              })
              .catch((e: unknown) => {
                const why = e instanceof Error ? e.message : String(e);
                core?.log('info', `not handed back: ${why}`);
                if (why.startsWith('not the game yet')) {
                  // A launcher hiding its window: the launcher wait (black, input locked) goes on;
                  // the core follows the game, or the wait's limit returns Home.
                  const g = games.find((x) => x.id === id);
                  if (g) veilText = `Starting ${g.name}…`;
                } else returnHome('AIONE is in front again');
              });
          });
          return;
        }
        returnHome('AIONE is in front again');
      };
      window.addEventListener('focus', onFocus);
      // D-056 (rev. bb): Alt + ` (the key under Esc, by position) caught by the page when the core's
      // hook didn't take it: a short Guide (back to the running game), through the same locks.
      // D-047: media / volume keys reaching the page go to the core's diagnostic (and its hint).
      const onKeyCapture = (e: KeyboardEvent) => {
        if (e.code === 'Backquote' && (e.altKey || e.type === 'keyup')) {
          e.preventDefault();
          e.stopImmediatePropagation();
          if (e.type === 'keydown' && !e.repeat) {
            core?.log('info', "Alt + ` reached AIONE's page (the core's hook didn't take it): Guide");
            handleInput({ source: 'keyboard', kind: 'down', control: 'Alt+`' });
          }
          return;
        }
        if (e.type === 'keydown' && (e.key.startsWith('Media') || e.key.startsWith('AudioVolume'))) {
          void core?.mediaKey(e.key).then((hint) => hint && showMessage(hint, 9000));
        }
      };
      window.addEventListener('keydown', onKeyCapture, { capture: true });
      window.addEventListener('keyup', onKeyCapture, { capture: true });
      unlisteners.push(() => {
        window.removeEventListener('keydown', onKeyCapture, { capture: true });
        window.removeEventListener('keyup', onKeyCapture, { capture: true });
      });
      // D-045: a key's release can't be seen from another window: clear the keyboard part of the
      // held set whenever AIONE gains or loses the front.
      const onFrontChange = () => void core?.clearKeys();
      window.addEventListener('focus', onFrontChange);
      window.addEventListener('blur', onFrontChange);
      unlisteners.push(() => {
        window.removeEventListener('focus', onFrontChange);
        window.removeEventListener('blur', onFrontChange);
      });
      unlisteners.push(() => window.removeEventListener('focus', onFocus));
      status = await c.statusNow();
      // The window is shown on its first frame, which is black (A9, D-010).
      requestAnimationFrame(() =>
        requestAnimationFrame(() => {
          c.uiReady().catch((err: unknown) => c.log('error', `ui_ready failed: ${String(err)}`));
        }),
      );
      // The black hold: the work that would make the Boot Up stutter happens now (alpha.6 QA-2:
      // the pictures are decoded ahead here, not during the animation), at most 4 s.
      const held = performance.now();
      const work = Promise.all([
        c.bootMedia().then(
          (m) => (bootMedia = m),
          (err: unknown) => c.log('error', `boot_media failed: ${String(err)}`),
        ),
        reloadLibrary(c),
        sounds.init().then(() => loadSettings(c)),
        loadFonts(c),
      ]);
      const hold = await Promise.race([work.then(() => 'done'), new Promise<string>((r) => setTimeout(() => r('timeout'), BLACK_HOLD_MAX_MS))]);
      c.log('info', `boot: black hold ${Math.round(performance.now() - held)} ms (${hold === 'done' ? 'library, pictures decoded ahead, fonts and boot media ready' : 'the 4 s limit was reached; the rest continues behind the Boot Up'})`);
      perf.start(() => [
        ...(keys.active ? ['keyboard repeat'] : []),
        'header clock (to the next minute)',
        ...(pressTimer ? ['home press'] : []),
      ]);
      // Tests (mocked build only): ?bootframe=up:2550 draws that moment of Window B and holds.
      const frameAt = import.meta.env.MODE === 'mock' ? params.get('bootframe') : null;
      if (frameAt) {
        const [which, t] = frameAt.split(':');
        phase = 'boot';
        bootPresenter(c)?.showFrame(which === 'down' ? 'down' : 'up', Number(t));
        return;
      }
      // D-014 (rev. r): Home is prepared during the black hold, BEFORE the Boot Up: mounted under
      // the black layer, its pictures decoded and its first frames drawn, so a skip at any moment
      // finds it ready.
      if (phase === 'black' && loadState === 'loaded') {
        const t0 = performance.now();
        prepareHome();
        await twoFrames();
        await Promise.all([...document.querySelectorAll<HTMLImageElement>('[data-testid="tile"] img')].map((i) => i.decode().catch(() => undefined)));
        await twoFrames();
        homeReadyAt = performance.now();
        c.log('info', `Home ready under the black screen in ${Math.round(homeReadyAt - t0)} ms, before the Boot Up`);
      }
      if (phase === 'black') await bootUp(c);
      if (phase !== 'bootdown') {
        // D-014: if Home isn't mounted yet (custom video, skip, no Boot Up), mount it under the
        // black layer and let it draw its first frames before the layer goes.
        if (!homeEarly) {
          prepareHome();
          await twoFrames();
          await Promise.all([...document.querySelectorAll<HTMLImageElement>('[data-testid="tile"] img')].map((i) => i.decode().catch(() => undefined)));
        }
        await twoFrames();
        phase = 'home';
        perf.setScreen('home');
        if (pendingTiles.length) c.log('info', `tile pictures: ${pendingTiles.length} copies made during the Boot Up are swapped in now`);
        pendingTiles.splice(0).forEach((t) => swapTile(c, t));
        try {
          const ms = await c.migrationStatus();
          flow = initialFlow(ms);
          // Rev. ba: the copy question once as a preview (the real offer if there is one).
          if (info?.showCopyQuestion && flow.phase !== 'ask') {
            previewCopy = true;
            const library = { games: 0, hidden: 0, favorites: 0, folders: 0, ignored: [], sources: [] };
            const offer = { state: 'offer' as const, source: '(a preview: nothing is copied)', files: 0, bytes: 0, library, notice: 'A preview (--show-copy-question): nothing is copied or recorded.' };
            flow = { phase: 'ask', offer, focus: BUTTON_COPY };
            c.log('info', 'copy question shown as a preview (--show-copy-question): nothing will be copied or recorded');
          } else if (info?.showCopyQuestion) {
            previewCopy = true;
          }
        } catch (err) {
          c.log('error', `migration_status failed: ${String(err)}`);
        }
      }
    })().catch((err: unknown) => console.error('startup failed', err));

    return () => {
      disposed = true;
      perf.stop();
      unlisteners.forEach((u) => u());
    };
  });

  /**
   * B-22: the input-locking state the UI shows, mirrored into the core (the one owner of states,
   * limits, the held set and the log). Changes are sent from here only.
   */
  const lockKind = $derived(
    phase === 'boot' ? 'boot-up'
    : phase === 'bootdown' ? 'boot-down'
    : launching === 'returning' ? 'returning'
    : launching === 'closing' ? 'closing'
    : launching === 'starting' && followingId ? 'launcher-wait'
    : launching === 'starting' ? 'launching'
    : confirm ? 'dialog'
    : po !== null ? 'dialog'
    : qm ? 'quick-menu'
    : 'none',
  );
  let lockSent = 'none';
  $effect(() => {
    const k = lockKind;
    if (k === lockSent) return;
    if (lockSent !== 'none') void core?.lockLeave(lockSent, lockHow);
    lockHow = 'done';
    if (k !== 'none') void core?.lockEnter(k, runningId ?? pendingId ?? null, `the UI: ${k}`);
    lockSent = k;
  });
</script>

<svelte:window
  onkeydown={onKey}
  onkeyup={onKey}
  onwheel={onWheel}
  onmousemove={onMouseMove}
  onresize={onResize}
  oncontextmenu={(e) => e.preventDefault()}
  onblur={() => {
    keys.releaseAll();
    presenter?.setForeground(false);
  }}
  onfocus={() => presenter?.setForeground(true)}
/>

<!-- D-010: black (the html background) until the library is loaded; then Home. -->
<main data-phase={phase} data-launching={launching} data-lock={lockKind} data-icons-ready={iconsReady} class:icons-ready={iconsReady} data-input={inputMode} data-testid="screen" data-version={info?.version ?? ''} data-library={library?.state ?? 'loading'}>
  <!-- A9: black from the first frame; the Boot Up / Boot Down draw here, over everything. -->
  <!-- Always mounted (the presenter keeps drawing into it); hidden on Home. -->
  <div
      class="boot"
      class:hidden={phase === 'home'}
      role="presentation"
      data-testid="boot-layer"
      data-phase={phase}
      bind:this={bootHost}
      onclick={() => {
        if (phase === 'boot' && presenter?.running) {
          core?.log('info', 'input click (mouse) -> Boot Up skipped');
          logSkip();
          presenter.skip();
        }
      }}
    ></div>
  {#if loadState !== 'loading' && (phase === 'home' || phase === 'bootdown' || homeEarly)}<Home
    {games}
    state={home}
    mode={inputMode}
    suspended={dialogOpen || !!confirm || !!qm || po !== null}
    {loadState}
    {device}
    padConnected={pads.pads.length > 0}
    {status}
    {pressed}
    {fixedTime}
    {...hour12 === undefined ? {} : { hour12 }}
    mediaUrl={(g, slot) => (core ? core.mediaUrl(g.id, slot, slot === 'tile' ? (g.tile ?? undefined) : undefined) : '')}
    onPicture={(what, w, h) => perf.picture(what, w, h)}
    {onHover}
    onClick={onTileClick}
    {onDrawerHover}
    {onDrawerClick}
    {drawerPressed}
    bump={bump && (bump.zone === 'row' || bump.zone === 'drawer') ? (bump as { dir: 'left' | 'right' | 'up' | 'down'; n: number; zone: 'row' | 'drawer' }) : null}
    {wrapping}
    {runningId}
    {startingId}
    onDragTick={() => play('scrollTick')}
    onDragEnd={(first) => {
      home = { ...home, first, hover: null };
      core?.log('info', `input drag (mouse / touch) -> the row at tile ${first + 1}, glide 150 ms`);
    }}
    onPrompt={(button) => {
      const action = button === 'close' ? 'close' : button;
      core?.log('info', `input click footer "${button}" (mouse) -> ${act(action, 'press')}`);
    }}
    scale={(info?.interfaceScale ?? 100) / 100}
    extras={extrasOn ? ['titlefade', 'press', 'labelfade'] : []}
  />{/if}
  {#if toast && phase === 'home'}
    <div class="toast" role="status" data-testid="toast">{toast}</div>
  {/if}

  {#if flow.phase === 'ask'}
    <!-- Rev. ba: T0's copy question on the shared dialog (the same design as every question). -->
    <ConfirmDialog
      title="Copy your AIONE data?"
      text={`AIONE 4 found the data of your previous AIONE: ${plural(flow.offer.files, 'file', 'files')}, ${formatBytes(flow.offer.bytes)}.`}
      details={[
        ...(flow.offer.notice ? [{ text: flow.offer.notice, testid: 'copy-notice', tone: 'warn' as const }] : []),
        { text: describeCounts(flow.offer.library), testid: 'copy-offer-counts' },
        { text: "It can copy everything into AIONE 4's own folder. Your previous AIONE folder is only read and stays exactly as it is. Not now (B) asks again at the next start; Copy and Don't copy are final." },
        ...describeSkipped(flow.offer.library.ignored).map((t) => ({ text: t, tone: 'muted' as const })),
        { text: flow.offer.source, tone: 'path' as const },
      ]}
      yes="Copy"
      no="Don't copy"
      focus={flow.focus === BUTTON_COPY ? 'yes' : 'no'}
      testid="copy-dialog"
      phase="ask"
      yesTestid="copy-accept"
      noTestid="copy-decline"
      hints={{ text: 'A Select · B Not now', testid: 'copy-hints' }}
      outMs={0}
      onAnswer={(y) => onChoose(y)}
    />
  {:else if flow.phase !== 'hidden'}
    <CopyDialog
      {flow}
      {onChoose}
      onClose={() => {
        core?.log('info', `input click "OK" (mouse) -> action close, ${play('confirm')}`);
        flow = { phase: 'hidden' };
      }}
    />
  {/if}
  {#if qm}
    <QuickMenu
      row={qm.row}
      brightness={bright}
      volume={uiVolume}
      muted={uiMuted}
      theme="Switch"
      {device}
      padConnected={pads.pads.length > 0}
      {status}
      {fixedTime}
      {glide}
      {...hour12 === undefined ? {} : { hour12 }}
      onRow={(r) => core?.log('info', `input click Quick Menu row ${r} -> ${qm ? ((qm = { row: r }), qmAct('confirm', 'press')) : ''}`)}
      onPrompt={(b) => core?.log('info', `input click Quick Menu footer "${b}" -> ${qmAct(b === 'back' ? 'back' : 'confirm', 'press')}`)}
    />
  {/if}
  {#if po !== null}
    <PowerOptions
      focus={po}
      {device}
      onPick={(i) => core?.log('info', `input click Power ${i} -> ${((po = i), powerItem(i, 'click'))}`)}
      onPrompt={(b) => core?.log('info', `input click Power options footer "${b}" -> ${poAct(b === 'back' ? 'back' : 'confirm')}`)}
    />
  {/if}
  {#if confirm}
    <ConfirmDialog title={confirm.title} text={confirm.text} yes={confirm.yes} focus={confirm.focus} single={confirm.single ?? false} onAnswer={(y) => core?.log('info', `input click question -> ${answer(y, 'click')}`)} />
  {/if}
  <!-- T1.3: the launch veil: Home fades to black (0.3 s, eased) after 0.55 s still. -->
  <div class="launch-veil" class:on={veil} data-testid="launch-veil" aria-hidden="true">
    {#if veil && closingSpinner}<div class="veil-ring" data-testid="closing-ring"></div>{/if}
    {#if veil && veilText}<p class="veil-text" data-testid="veil-text">{veilText}{#if launching === 'starting' && launchBackOk && !followingId}<span class="veil-b">  B Cancel start</span>{/if}</p>{/if}
  </div>
</main>

<style>
  .launch-veil {
    position: fixed;
    inset: 0;
    background: #000;
    opacity: 0;
    pointer-events: none;
    transition: opacity 300ms ease-in-out; /* rev. ab: about 0.3 s, eased */
    z-index: 900;
  }
  .veil-text {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 12%;
    margin: 0;
    text-align: center;
    color: rgb(255 255 255 / 0.75);
    font: 400 var(--text-sm) var(--font-ui);
  }
  /* B-18 (rev. am): the small progress ring on "Closing <game>…". */
  .veil-ring {
    position: absolute;
    left: 50%;
    bottom: calc(12% + 2.6em);
    width: 1.6em;
    height: 1.6em;
    margin-left: -0.8em;
    border-radius: 50%;
    border: 0.18em solid rgb(255 255 255 / 0.25);
    border-top-color: rgb(255 255 255 / 0.85);
    animation: veil-spin 0.9s linear infinite;
  }
  @keyframes veil-spin {
    to {
      transform: rotate(360deg);
    }
  }
  .veil-b {
    margin-left: 1.5em;
    opacity: 0.85;
    white-space: pre;
  }
  .launch-veil.on {
    opacity: 1;
    pointer-events: auto;
  }
  main {
    position: relative;
    height: 100%;
  }
  /* Icons only with their font (never their ligature names as text). */
  main:not(.icons-ready) :global(.icon) {
    visibility: hidden;
  }
  /* T1.2: a plain toast in the footer band's empty centre (between the branding and the
     prompts), so it never covers the drawer's label. */
  .toast {
    position: fixed;
    left: 50%;
    bottom: 1.1rem;
    transform: translateX(-50%);
    z-index: 50;
    padding: 0.6rem 1.4rem;
    line-height: 1.15;
    border-radius: 0.75rem;
    background: var(--color-surface-raised);
    color: var(--color-text);
    border: 1px solid var(--color-divider);
    font-size: var(--text-label);
    white-space: nowrap;
  }
  .boot.hidden {
    display: none;
  }
  .boot {
    position: fixed;
    inset: 0;
    z-index: 100;
    background: #000;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
  }
  /* The mark keeps the reference page's proportions: on a 1920 × 1080 window the reference draws
     its 680 × 460 view 744 px tall, i.e. 69 % of the height. Nothing clips the shake, ring or
     shards (the layer and the SVG show their overflow). */
  .boot :global(svg) {
    height: 69%;
    width: auto;
    aspect-ratio: 680 / 460;
    overflow: visible;
  }
</style>
