<script lang="ts">
  // AIONE 4 (T1.1): Home with the copy dialog over it when needed. This component owns input
  // (A6): every controller, keyboard and mouse event arrives here once, becomes one action, and
  // goes to the active context only: the dialog when it is open, else Home.
  import { onMount } from 'svelte';
  import { loadCore } from './lib/core';
  import type { AppInfo, Core, GameView, LibraryLoad, PcStatus, Unlisten } from './lib/core/types';
  import {
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
  import CopyDialog from './lib/components/CopyDialog.svelte';

  /** Mouse movement that counts as deliberate (A7): beyond this many CSS pixels. */
  const MOUSE_THRESHOLD_PX = 6;
  const params = new URLSearchParams(globalThis.location?.search ?? '');

  let core: Core | null = null;
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

  function play(name: SoundName, kind: 'press' | 'repeat' = 'press'): string {
    return sounds.play(name, kind) ? `sound ${name}` : name === 'move' && kind === 'repeat' ? 'sound skipped (repeat pacing)' : 'no sound';
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
    perf.input(ev.source);
    sounds.resume();
    if (ev.source !== 'mouse') {
      device = deviceAfter(device, ev.source);
      setMode('pad');
    }
    const action = toAction(ev);
    const outcome = action ? act(action, ev.kind === 'repeat' ? 'repeat' : 'press') : undefined;
    core?.log('info', describeInput(ev, action, outcome));
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

  function setLibrary(l: LibraryLoad) {
    library = l;
    games = l.games.filter((g) => !g.hidden);
    loadState = l.state === 'error' ? 'error' : games.length > 0 ? 'loaded' : 'empty';
    home = withCount(home, games.length);
    core?.log('info', `home shows ${games.length} games (${l.games.length - games.length} hidden)`);
  }

  async function reloadLibrary(c: Core) {
    try {
      setLibrary(await c.libraryLoad());
    } catch (err) {
      loadState = 'error';
      c.log('error', `library_load failed: ${String(err)}`);
    }
  }

  $effect(() => {
    perf.setScreen(dialogOpen ? 'home + copy dialog' : 'home');
  });

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
      status = await c.statusNow();
      await reloadLibrary(c);
      void sounds.init();
      perf.start(() => [
        ...(keys.active ? ['keyboard repeat'] : []),
        'header clock (to the next minute)',
        ...(pressTimer ? ['home press'] : []),
      ]);
      // Show the window only once the first real frame is drawn, then the core takes the
      // foreground (L-20).
      requestAnimationFrame(() =>
        requestAnimationFrame(() => {
          c.uiReady().catch((err: unknown) => c.log('error', `ui_ready failed: ${String(err)}`));
        }),
      );
      try {
        flow = initialFlow(await c.migrationStatus());
      } catch (err) {
        c.log('error', `migration_status failed: ${String(err)}`);
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
  onblur={() => keys.releaseAll()}
/>

<main data-input={inputMode} data-testid="screen" data-version={info?.version ?? ''} data-library={library?.state ?? 'loading'}>
  <Home
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
    mediaUrl={(g) => (core && g.tileArt ? core.mediaUrl(g.id, g.tileArt) : '')}
    {onHover}
    onClick={onTileClick}
  />

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
    background: var(--color-bg);
  }
</style>
