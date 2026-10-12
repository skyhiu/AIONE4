<script lang="ts">
  import type { ReadoutState } from '../input/readout';
  import { describeLast } from '../input/readout';

  let { state }: { state: ReadoutState } = $props();

  const padsText = $derived(
    state.pads.length === 0
      ? 'No controller connected'
      : `${state.pads.length === 1 ? 'Controller' : 'Controllers'} connected: ${state.pads.map((p) => `slot ${p}`).join(', ')}`,
  );
</script>

<section class="readout" aria-label="Input check" data-testid="readout">
  <h2>Input check <span class="stage">T0 only</span></h2>
  <div class="last" aria-live="polite">
    <span class="label">Last press</span>
    {#key state.presses}
      <span class="value flash" data-testid="readout-last">{describeLast(state)}</span>
    {/key}
  </div>
  <dl>
    <div><dt>Presses</dt><dd data-testid="readout-presses">{state.presses}</dd></div>
    <div><dt>Repeats</dt><dd data-testid="readout-repeats">{state.repeats}</dd></div>
    <div><dt>Releases</dt><dd data-testid="readout-releases">{state.releases}</dd></div>
  </dl>
  <p class="sources" data-testid="readout-sources">
    Controller {state.bySource.controller} · Keyboard {state.bySource.keyboard} · Mouse {state.bySource.mouse}
  </p>
  <p class="pads" data-testid="readout-pads">{padsText}</p>
</section>

<style>
  .readout {
    width: min(30rem, 40vw);
    padding: var(--space-4) var(--space-5);
    background: var(--color-surface);
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
  }
  h2 {
    margin: 0 0 var(--space-3);
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--color-text);
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
  }
  .stage {
    font-size: var(--text-xs);
    font-weight: 400;
    color: var(--color-text-muted);
  }
  .last {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    margin-bottom: var(--space-3);
  }
  .label,
  dt {
    color: var(--color-text-muted);
    font-size: var(--text-xs);
  }
  .value {
    font-size: var(--text-md);
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .flash {
    animation: flash var(--motion-medium) var(--ease-out);
  }
  @keyframes flash {
    from {
      opacity: 0.35;
      transform: translateY(2px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
  dl {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--space-3);
    margin: 0 0 var(--space-3);
  }
  dd {
    margin: var(--space-1) 0 0;
    font-size: var(--text-md);
    font-variant-numeric: tabular-nums;
  }
  .sources,
  .pads {
    margin: var(--space-1) 0 0;
    color: var(--color-text-muted);
    font-size: var(--text-xs);
  }
</style>
