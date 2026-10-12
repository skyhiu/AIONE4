<script lang="ts">
  import { fade, scale } from 'svelte/transition';
  import type { FlowState } from '../migrationFlow';
  import { BUTTON_COPY, BUTTON_SKIP } from '../migrationFlow';
  import { formatBytes, plural } from '../format';

  let {
    flow,
    onChoose,
    onClose,
  }: {
    flow: Exclude<FlowState, { phase: 'hidden' }>;
    onChoose: (accept: boolean) => void;
    onClose: () => void;
  } = $props();

  const progressFraction = $derived(
    flow.phase === 'copying' && flow.progress && flow.progress.bytesTotal > 0
      ? Math.min(1, flow.progress.bytesDone / flow.progress.bytesTotal)
      : 0,
  );
</script>

<div class="scrim" transition:fade={{ duration: 160 }}></div>
<div
  class="dialog"
  role="dialog"
  aria-modal="true"
  aria-labelledby="copy-title"
  data-testid="copy-dialog"
  data-phase={flow.phase}
  transition:scale={{ duration: 200, start: 0.96 }}
>
  {#if flow.phase === 'ask'}
    <h1 id="copy-title">Copy your AIONE data?</h1>
    <p>
      AIONE 4 found the data of your previous AIONE: {plural(flow.offer.library.games, 'game', 'games')}, {plural(
        flow.offer.files,
        'file',
        'files',
      )}, {formatBytes(flow.offer.bytes)}.
    </p>
    <p>
      It can copy everything into AIONE 4's own folder. Your previous AIONE folder is only read and stays exactly as
      it is. This question is asked only once.
    </p>
    <p class="path" title={flow.offer.source}>{flow.offer.source}</p>
    <div class="buttons">
      <button
        class="focusable"
        class:focused={flow.focus === BUTTON_COPY}
        data-testid="copy-accept"
        onclick={() => onChoose(true)}>Copy</button
      >
      <button
        class="focusable"
        class:focused={flow.focus === BUTTON_SKIP}
        data-testid="copy-decline"
        onclick={() => onChoose(false)}>Don't copy</button
      >
    </div>
  {:else if flow.phase === 'copying'}
    <h1 id="copy-title">Copying your AIONE data…</h1>
    <p data-testid="copy-progress-text">
      {#if flow.progress}
        {flow.progress.filesDone.toLocaleString('en-US')} of {plural(flow.progress.filesTotal, 'file', 'files')} ·
        {formatBytes(flow.progress.bytesDone)} of {formatBytes(flow.progress.bytesTotal)}
      {:else}
        Preparing…
      {/if}
    </p>
    <div class="bar" aria-hidden="true"><div class="fill" style:transform={`scaleX(${progressFraction})`}></div></div>
    <p class="muted">Your previous AIONE folder is only read, never changed.</p>
  {:else}
    {@const r = flow.result}
    <h1 id="copy-title">{r.ok ? 'Your AIONE data was copied' : 'The copy is incomplete'}</h1>
    <p data-testid="copy-result-games">
      {plural(r.library.games, 'game', 'games')} ({r.library.hidden} hidden, {r.library.favorites}
      {r.library.favorites === 1 ? 'favorite' : 'favorites'}) · {plural(r.filesCopied, 'file', 'files')} ·
      {formatBytes(r.bytesCopied)}
    </p>
    {#if r.library.ignored.length > 0}
      <p class="muted" data-testid="copy-result-ignored">
        Not games (as in AIONE): {r.library.ignored.map((i) => `${i.name}: ${i.reason}`).join('; ')}
      </p>
    {/if}
    {#if r.filesFailed > 0}
      <p class="warn">{plural(r.filesFailed, 'file', 'files')} could not be copied (listed in aione.log).</p>
    {/if}
    {#each r.notes as note (note)}
      <p class="muted">{note}</p>
    {/each}
    <p class={r.sourceUnchanged ? 'ok' : 'warn'} data-testid="copy-result-source">
      {r.sourceUnchanged
        ? 'Your previous AIONE folder was not changed.'
        : 'Your previous AIONE folder changed during the copy (another program wrote to it); see aione.log.'}
    </p>
    <div class="buttons">
      <button class="focusable focused" data-testid="copy-ok" onclick={onClose}>OK</button>
    </div>
  {/if}
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.55);
  }
  .dialog {
    position: fixed;
    left: 50%;
    top: 50%;
    translate: -50% -50%;
    width: min(44rem, 88vw);
    max-height: 86vh;
    overflow: hidden;
    padding: var(--space-6) var(--space-6) var(--space-5);
    background: var(--color-surface-raised);
    border-radius: var(--radius-lg);
    box-shadow: 0 18px 48px rgb(0 0 0 / 0.45);
  }
  h1 {
    margin: 0 0 var(--space-4);
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: 600;
  }
  p {
    margin: 0 0 var(--space-3);
    font-size: var(--text-sm);
    line-height: 1.5;
    overflow-wrap: anywhere;
  }
  .path {
    font-size: var(--text-xs);
    color: var(--color-text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .muted {
    color: var(--color-text-muted);
    font-size: var(--text-xs);
  }
  .ok {
    color: var(--color-ok);
  }
  .warn {
    color: var(--color-warn);
  }
  .buttons {
    display: flex;
    gap: var(--space-4);
    margin-top: var(--space-5);
  }
  button {
    flex: 1;
    min-height: 3.25rem;
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    color: var(--color-text);
    font: 600 var(--text-sm) var(--font-ui);
    outline: var(--ring-width) solid transparent;
    outline-offset: 3px;
    transition:
      transform var(--motion-fast) var(--ease-out),
      opacity var(--motion-fast) var(--ease-out);
    cursor: pointer;
  }
  button.focused {
    outline-color: var(--color-ring);
  }
  :global([data-input='mouse']) button.focused {
    outline-color: transparent;
  }
  :global([data-input='mouse']) button:hover {
    background: var(--color-divider);
  }
  button:active {
    transform: scale(0.97);
  }
  .bar {
    height: 8px;
    border-radius: 4px;
    background: var(--color-surface);
    overflow: hidden;
    margin: var(--space-4) 0;
  }
  .fill {
    height: 100%;
    background: var(--color-accent);
    transform-origin: left center;
    transition: transform var(--motion-medium) linear;
  }
</style>
