<script lang="ts">
  // B-18: a plain two-button question ("Close and start" / "Cancel"), Cancel focused by default.
  // The pad drives it from App (Left / Right move, A answers the focused button, B cancels).
  import { fade, scale } from 'svelte/transition';

  let {
    title,
    text,
    yes,
    no = 'Cancel',
    focus,
    single = false,
    testid = 'confirm-dialog',
    phase = undefined,
    yesTestid = 'confirm-yes',
    noTestid = 'confirm-no',
    details = [],
    hints = undefined,
    outMs = 200,
    onAnswer,
  }: {
    title: string;
    text: string;
    yes: string;
    no?: string;
    focus: 'yes' | 'no';
    /** L-13 / L-18 (T2.2): one component for every question and message; a message has one button. */
    single?: boolean;
    /** Rev. ba: optional parts (T0's copy question): ids, detail lines, a hint line. */
    testid?: string;
    phase?: string;
    yesTestid?: string;
    noTestid?: string;
    details?: { text: string; testid?: string; tone?: 'warn' | 'muted' | 'path' }[];
    hints?: { text: string; testid: string };
    /** How long it takes to leave (0: replaced at once, e.g. by the copy's progress). */
    outMs?: number;
    onAnswer: (yes: boolean) => void;
  } = $props();
</script>

<div class="scrim" in:fade={{ duration: 160 }} out:fade={{ duration: Math.min(160, outMs) }}></div>
<div class="dialog" role="alertdialog" aria-modal="true" aria-labelledby="confirm-title" data-testid={testid} data-phase={phase} in:scale={{ duration: 200, start: 0.96 }} out:scale={{ duration: outMs, start: 0.96 }}>
  <h1 id="confirm-title">{title}</h1>
  <p data-testid="confirm-text">{text}</p>
  {#each details as d, i (i)}
    <p class="detail {d.tone ?? ''}" data-testid={d.testid} title={d.tone === 'path' ? d.text : undefined}>{d.text}</p>
  {/each}
  <div class="buttons">
    {#if single}
      <button class="focusable focused" data-testid="confirm-ok" onclick={() => onAnswer(false)}>{yes}<span class="ring" aria-hidden="true"></span></button>
    {:else}
      <button class="focusable" class:focused={focus === 'yes'} data-testid={yesTestid} onclick={() => onAnswer(true)}>{yes}<span class="ring" aria-hidden="true"></span></button>
      <button class="focusable" class:focused={focus === 'no'} data-testid={noTestid} onclick={() => onAnswer(false)}>{no}<span class="ring" aria-hidden="true"></span></button>
    {/if}
  </div>
  {#if hints}<p class="hints" data-testid={hints.testid}>{hints.text}</p>{/if}
</div>

<style>
  .detail {
    margin-top: 0.4em;
  }
  .detail.warn {
    color: var(--color-accent);
  }
  .detail.muted,
  .detail.path,
  .hints {
    color: var(--color-text-muted);
    font-size: 0.85em;
  }
  .detail.path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .scrim {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.55);
    z-index: 950;
  }
  .dialog {
    position: fixed;
    left: 50%;
    top: 50%;
    translate: -50% -50%;
    width: min(44rem, 88vw);
    padding: var(--space-6) var(--space-6) var(--space-5);
    background: var(--color-surface-raised);
    color: var(--color-text);
    border-radius: var(--radius-lg);
    box-shadow: 0 18px 48px rgb(0 0 0 / 0.45);
    z-index: 951;
  }
  h1 {
    margin: 0 0 var(--space-4);
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: 600;
    overflow-wrap: anywhere;
  }
  p {
    margin: 0;
    font-size: var(--text-sm);
    line-height: 1.5;
    overflow-wrap: anywhere;
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
    position: relative;
    cursor: pointer;
  }
  /* B-29: the focus is a ring overlay (the same look as the old outline), so the edge bump can
     move the ring while the button stays still. */
  .ring {
    position: absolute;
    inset: calc(-3px - var(--ring-width));
    border: var(--ring-width) solid transparent;
    border-radius: calc(var(--radius-sm) + 3px);
    pointer-events: none;
  }
  button.focused .ring {
    border-color: var(--color-ring);
  }
  :global([data-input='mouse']) button.focused .ring {
    border-color: transparent;
  }
  :global([data-input='mouse']) button:hover {
    background: var(--color-soft);
    color: var(--color-on-soft);
  }
</style>
