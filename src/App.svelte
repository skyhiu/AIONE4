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
  import { MenuSounds, type SoundName } from './lib/audio/sounds';
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
  let bootMedia: BootMedia | null = null;
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

  /** The log wording for one sound request (QA-6: says what really happened). */
  function play(name: SoundName, kind: 'press' | 'repeat' = 'press'): string {
    const r = sounds.play(name, kind);
    if (r === 'played') return `sound ${name}`;
    if (r === 'paced') return 'sound skipped (repeat pacing)';
    return 'no sound (sounds unavailable)';
  }

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
    if (ev.kind === 'up') return;
    // A9: the input lock. During the black hold, Boot Up and Boot Down nothing in AIONE is
    // reachable; input is discarded (never queued). The only input is the Boot Up's skip.
    if (phase !== 'home') {
      const action = toAction(ev);
      if (phase === 'boot' && ev.kind === 'down' && (action === 'confirm' || action === 'back') && presenter?.running) {
        core?.log('info', `${describeInput(ev, action, 'Boot Up skipped')}`);
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
            ? `on tile ${home.focus + 1}`
            : 'on Home';
        core?.log('info', `input ${ev.kind} ${ev.control} (${inputWhere(ev)}) -> ring back ${where} (after mouse), no move, no sound${waited}`);
        core?.log('info', `ring back ${where} (after mouse)`);
        perf.action(`ring back ${where}`);
        return;
      }
    }
    const action = toAction(ev);
    const outcome = action ? act(action, ev.kind === 'repeat' ? 'repeat' : 'press') : undefined;
    perf.action(`${ev.kind} ${ev.control} → ${action ?? 'no action'}${loadState === 'loaded' && !dialogOpen ? `, focus ${home.focus + 1}/${games.length} "${games[home.focus]?.name ?? ''}"` : ''}`);
    core?.log('info', describeInput(ev, action, `${outcome ?? ''}${waited}`.replace(/^, /, '')));
  }

  /** One action to the active context. Returns what it did, for the log line. */
  function act(action: UiAction, kind: 'press' | 'repeat'): string {
    if (dialogOpen) {
      const before = flow;
      const [next, effect] = flowAction(flow, action);
      flow = next;
      applyEffect(effect);
      if (effect.type === 'decide') return `dialog decide, ${play('select')}`;
      if (effect.type === 'notNow') return `dialog not now, ${play('back')}`;
      if (next.phase === 'hidden') return `dialog closed, ${play(action === 'back' ? 'back' : 'select')}`;
      const moved = 'focus' in before && 'focus' in next && before.focus !== next.focus;
      return moved ? `dialog focus moved, ${play('move', kind)}` : 'dialog: nothing to do';
    }
    if (loadState !== 'loaded') return 'home: no games';
    switch (action) {
      case 'left':
      case 'right': {
        const [next, moved] = move(home, action === 'left' ? -1 : 1);
        home = next;
        return moved ? `focus ${home.focus + 1}/${games.length}, ${play('move', kind)}` : 'end of row, no sound';
      }
      case 'confirm':
        return start(home.focus, 'press');
      case 'back':
        return 'home: nothing to go back to';
      default:
        return 'home: nothing there yet (the drawer comes in T1.2)';
    }
  }

  /** Starting a game comes in T1.3: for now the press is shown, sounded and logged. */
  function start(index: number, how: string): string {
    const g = games[index];
    if (!g) return 'nothing to start';
    pressFeedback(index);
    const s = play('select');
    core?.log('info', `start requested: "${g.name}" (${how}); launching comes in T1.3`);
    return `start "${g.name}", ${s}`;
  }

  function pressFeedback(index: number) {
    pressed = index;
    if (pressTimer) clearTimeout(pressTimer);
    pressTimer = setTimeout(() => {
      pressed = null;
      pressTimer = null;
    }, 90);
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
    core?.log('info', `input click "${accept ? 'Copy' : "Don't copy"}" (mouse) -> action ${accept ? 'copy' : 'dont-copy'}, ${play('select')}`);
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
      core?.log('info', `input click tile ${index + 1} "${g?.name ?? ''}" (mouse) -> select (first click), ${play('move')}`);
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
      core?.log('info', 'restart requested (Ctrl+Shift+R)');
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
        outcome = await p.playDefault('up', bootUpSound);
      }
    } else {
      c.log('info', `Boot Up: default Window B (${info?.bootDefault ? '--boot=default for this run' : 'no custom video'})`);
      outcome = await p.playDefault('up', bootUpSound);
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
      ]);
      const hold = await Promise.race([work.then(() => 'done'), new Promise<string>((r) => setTimeout(() => r('timeout'), BLACK_HOLD_MAX_MS))]);
      c.log('info', `boot: black hold ${Math.round(performance.now() - held)} ms (${hold === 'done' ? 'library, pictures decoded ahead and boot media ready' : 'the 4 s limit was reached; the rest continues behind the Boot Up'})`);
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
      if (phase === 'black') await bootUp(c);
      if (phase !== 'bootdown') {
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
<main data-input={inputMode} data-testid="screen" data-version={info?.version ?? ''} data-library={library?.state ?? 'loading'}>
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
          presenter.skip();
        }
      }}
    ></div>
  {#if loadState !== 'loading' && (phase === 'home' || phase === 'bootdown')}<Home
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
  />{/if}

  {#if flow.phase !== 'hidden'}
    <CopyDialog
      {flow}
      {onChoose}
      onClose={() => {
        core?.log('info', `input click "OK" (mouse) -> action close, ${play('select')}`);
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
