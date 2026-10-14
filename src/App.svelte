<script lang="ts">
  // AIONE 4 (T1.1): Home with the copy dialog over it when needed. This component owns input
  // (A6): every controller, keyboard and mouse event arrives here once, becomes one action, and
  // goes to the active context only: the dialog when it is open, else Home.
  import { onMount } from 'svelte';
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
  import { choose, flowAction, flowDone, flowProgress, initialFlow, type FlowEffect, type FlowState } from './lib/migrationFlow';
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
  import bootUpSound from '../assets/sounds/boot/aione_boot_up.wav?url';
  import bootDownSound from '../assets/sounds/boot/aione_boot_down.wav?url';
  import type { BootMedia } from './lib/core/types';
  import CopyDialog from './lib/components/CopyDialog.svelte';

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
  let bump = $state<{ dir: 'left' | 'right' | 'up' | 'down'; n: number; zone: 'row' | 'drawer' } | null>(null);
  /** D-019: the directions that already bumped in their current hold (one memory per
   *  direction, cleared when that direction is released). */
  const bumpedHolds: Record<string, boolean> = {};
  /** B-14 (rev. q / s / u): a blocked move bumps the ring only, with the edge-bump sound (instead
   *  of the click, never with it); once per held direction. */
  function edge(dir: 'left' | 'right' | 'up' | 'down', kind: 'press' | 'repeat', what: string): string {
    if (kind === 'repeat' && bumpedHolds[dir]) return `${what}, held: no bump`;
    bumpedHolds[dir] = true;
    bump = { dir, n: (bump?.n ?? 0) + 1, zone: home.zone };
    return `${what}, bump ${dir}, ${play('edgeBump')}`;
  }
  /** B-14 (rev. u): a press more than about 0.4 s after the previous move is "resting": at an end
   *  it wraps at once; sooner (fast scrolling) or a held repeat bumps. */
  const RESTING_MS = 400;
  let lastMoveAt = 0;
  /** The row whips to its other end in 200 ms (its normal slide is 170 ms). */
  let wrapping = $state(false);
  let wrapTimer: ReturnType<typeof setTimeout> | null = null;
  /** B-1 (rev. s): during a hold the row-move repeats get quieter after the first few. */
  let holdRepeats = 0;
  const QUIET_AFTER = 3;
  const QUIET_DB = -4;

  function onKeyPhase(ev: InputEvent, release?: ReleaseInfo) {
    if (ev.kind === 'up') {
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
    if (dialogOpen) {
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
        const sinceMove = performance.now() - lastMoveAt;
        if (moved) {
          // B-1 (rev. u): when the row itself scrolls, its low tick joins the click.
          const scrolled = next.first !== home.first;
          home = next;
          lastMoveAt = performance.now();
          holdRepeats = kind === 'repeat' ? holdRepeats + 1 : 0;
          const quiet = holdRepeats > QUIET_AFTER ? QUIET_DB : 0;
          return `${where(home.focus)}${scrolled ? ' (row scrolls)' : ''}, ${play(scrolled ? 'rowScroll' : 'rowMove', quiet)}`;
        }
        // B-14 (rev. u): at the row's end. Held, or fast scrolling (the previous move under about
        // 0.4 s ago): a ring bump, once per hold. A resting press: the wrap at once, no bump.
        if (kind === 'repeat' || sinceMove <= RESTING_MS) return edge(action, kind, `end of row${kind === 'repeat' ? '' : ` (${Math.round(sinceMove)} ms after the last move)`}`);
        wrapping = true;
        if (wrapTimer) clearTimeout(wrapTimer);
        wrapTimer = setTimeout(() => {
          wrapping = false;
          wrapTimer = null;
        }, 260);
        home = wrapRow(home, dir);
        lastMoveAt = performance.now();
        return `wrap to ${dir > 0 ? 'the first game' : 'the last slot'} (resting ${Math.round(sinceMove)} ms): ${where(home.focus)}, ${play('wrap')}`;
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
  function start(index: number, how: string): string {
    const g = games[index];
    if (!g) return 'nothing to start';
    pressFeedback(index);
    const s = play('confirm');
    core?.log('info', `start requested: "${g.name}" (${how}); launching comes in T1.3`);
    return `start "${g.name}", ${s}`;
  }

  function pressFeedback(index: number) {
    pressed = index;
    if (pressTimer) clearTimeout(pressTimer);
    pressTimer = setTimeout(() => {
      pressed = null;
      pressTimer = null;
    }, extrasOn ? 200 : 90);
  }

  function applyEffect(effect: FlowEffect) {
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
  async function bootDown(c: Core) {
    if (phase === 'bootdown') return;
    phase = 'bootdown';
    perf.setScreen('Boot Down');
    perf.action('Boot Down');
    c.log('info', 'Close AIONE: the Boot Down starts (input locked)');
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
        sounds.init(),
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
          flow = initialFlow(await c.migrationStatus());
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
<main data-phase={phase} data-icons-ready={iconsReady} class:icons-ready={iconsReady} data-input={inputMode} data-testid="screen" data-version={info?.version ?? ''} data-library={library?.state ?? 'loading'}>
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
    suspended={dialogOpen}
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
    {bump}
    {wrapping}
    scale={(info?.interfaceScale ?? 100) / 100}
    extras={extrasOn ? ['titlefade', 'press', 'labelfade'] : []}
  />{/if}
  {#if toast && phase === 'home'}
    <div class="toast" role="status" data-testid="toast">{toast}</div>
  {/if}

  {#if flow.phase !== 'hidden'}
    <CopyDialog
      {flow}
      {onChoose}
      onClose={() => {
        core?.log('info', `input click "OK" (mouse) -> action close, ${play('confirm')}`);
        flow = { phase: 'hidden' };
      }}
    />
  {/if}
</main>

<style>
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
