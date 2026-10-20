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
    onAnswer,
  }: {
    title: string;
    text: string;
    yes: string;
    no?: string;
    focus: 'yes' | 'no';
    /** L-13 / L-18 (T2.2): one component for every question and message; a message has one button. */
    single?: boolean;
    onAnswer: (yes: boolean) => void;
  } = $props();
</script>

<div class="scrim" transition:fade={{ duration: 160 }}></div>
<div class="dialog" role="alertdialog" aria-modal="true" aria-labelledby="confirm-title" data-testid="confirm-dialog" transition:scale={{ duration: 200, start: 0.96 }}>
  <h1 id="confirm-title">{title}</h1>
  <p data-testid="confirm-text">{text}</p>
  <div class="buttons">
    {#if single}
      <button class="focusable focused" data-testid="confirm-ok" onclick={() => onAnswer(false)}>{yes}</button>
    {:else}
      <button class="focusable" class:focused={focus === 'yes'} data-testid="confirm-yes" onclick={() => onAnswer(true)}>{yes}</button>
      <button class="focusable" class:focused={focus === 'no'} data-testid="confirm-no" onclick={() => onAnswer(false)}>{no}</button>
    {/if}
  </div>
</div>

<style>
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
    outline: var(--ring-width) solid transparent;
    outline-offset: 3px;
    cursor: pointer;
  }
  button.focused {
    outline-color: var(--color-ring);
  }
  :global([data-input='mouse']) button.focused {
    outline-color: transparent;
  }
  :global([data-input='mouse']) button:hover {
    background: var(--color-soft);
    color: var(--color-on-soft);
  }
</style>
