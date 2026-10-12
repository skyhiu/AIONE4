<script lang="ts">
  import { onMount } from 'svelte';
  import { loadCore } from './lib/core';
  import type { AppInfo, Core, Unlisten } from './lib/core/types';
  import {
    fromKeyboard,
    fromMouseButton,
    fromPadEvent,
    fromWheel,
    toAction,
    type InputEvent,
  } from './lib/input/events';
  import { applyInput, applyPadConnection, initialReadout } from './lib/input/readout';
  import { choose, flowAction, flowDone, flowProgress, initialFlow, type FlowState } from './lib/migrationFlow';
  import { isBlockedBrowserKey, isRestartShortcut } from './lib/input/shortcuts';
  import InputReadout from './lib/components/InputReadout.svelte';
  import CopyDialog from './lib/components/CopyDialog.svelte';

  let core: Core | null = null;
  let info = $state<AppInfo | null>(null);
  let readout = $state(initialReadout());
  let flow = $state<FlowState>({ phase: 'hidden' });
  /** A7: 'pad' = controller / keyboard (focus ring), 'mouse' = hover only. */
  let inputMode = $state<'pad' | 'mouse'>('pad');

  function handleInput(ev: InputEvent) {
    readout = applyInput(readout, ev);
    if (ev.kind === 'down') inputMode = ev.source === 'mouse' ? 'mouse' : 'pad';
    const action = toAction(ev);
    if (!action || !core) return;
    const [next, effect] = flowAction(flow, action);
    flow = next;
    if (effect.type === 'decide') decide(effect.accept);
  }

  function decide(accept: boolean) {
    core?.log('info', `copy offer: user chose ${accept ? 'Copy' : "Don't copy"}`);
    core?.migrationDecide(accept).catch((err: unknown) => {
      core?.log('error', `migration_decide failed: ${String(err)}`);
    });
  }

  function onChoose(accept: boolean) {
    const [next, effect] = choose(flow, accept);
    flow = next;
    if (effect.type === 'decide') decide(effect.accept);
  }

  function onKey(e: KeyboardEvent) {
    if (isBlockedBrowserKey(e)) e.preventDefault();
    if (e.type === 'keydown' && isRestartShortcut(e) && !e.repeat) {
      e.preventDefault();
      core?.log('info', 'restart requested (Ctrl+Shift+R)');
      core?.restart().catch((err: unknown) => core?.log('error', `restart failed: ${String(err)}`));
      return;
    }
    handleInput(fromKeyboard(e, e.type === 'keyup' ? 'keyup' : 'keydown'));
  }

  function onMouse(e: MouseEvent) {
    handleInput(fromMouseButton(e.button, e.type === 'mouseup' ? 'mouseup' : 'mousedown'));
  }

  function onWheel(e: WheelEvent) {
    const ev = fromWheel(e.deltaY);
    if (ev) handleInput(ev);
  }

  function onMouseMove() {
    if (inputMode !== 'mouse') inputMode = 'mouse';
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
            readout = applyPadConnection(readout, ev.pad + 1, ev.kind === 'connected');
            return;
          }
          const input = fromPadEvent(ev);
          if (input) handleInput(input);
        }),
        await c.onCopyProgress((p) => (flow = flowProgress(flow, p))),
        await c.onCopyDone((r) => (flow = flowDone(flow, r))),
      );
      info = await c.appInfo();
      // Show the window only once the first real frame is drawn (no white flash), then
      // the core takes the foreground (L-20).
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
      unlisteners.forEach((u) => u());
    };
  });
</script>

<svelte:window
  onkeydown={onKey}
  onkeyup={onKey}
  onmousedown={onMouse}
  onmouseup={onMouse}
  onwheel={onWheel}
  onmousemove={onMouseMove}
  oncontextmenu={(e) => e.preventDefault()}
/>

<main data-input={inputMode} data-testid="screen">
  <div class="brand">
    <h1 data-testid="wordmark">AIONE 4</h1>
    <p class="version" data-testid="version">{info ? `Version ${info.version}` : ''}</p>
  </div>

  <div class="bottom">
    <InputReadout state={readout} />
    <p class="hints">Ctrl+Shift+R Restart · Alt+F4 Close</p>
  </div>

  {#if flow.phase !== 'hidden'}
    <CopyDialog {flow} {onChoose} onClose={() => (flow = { phase: 'hidden' })} />
  {/if}
</main>

<style>
  main {
    position: relative;
    height: 100%;
    background: var(--color-bg);
    display: grid;
    grid-template-rows: 1fr auto;
    padding: var(--space-7);
  }
  .brand {
    align-self: center;
    justify-self: center;
    text-align: center;
  }
  h1 {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--text-display);
    font-weight: 600;
    letter-spacing: 0.02em;
    line-height: 1.05;
  }
  .version {
    margin: var(--space-3) 0 0;
    min-height: 1.5em;
    font-size: var(--text-md);
    color: var(--color-text-muted);
    font-variant-numeric: tabular-nums;
  }
  .bottom {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: var(--space-5);
  }
  .hints {
    margin: 0;
    color: var(--color-text-muted);
    font-size: var(--text-xs);
  }
</style>
