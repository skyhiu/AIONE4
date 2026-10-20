<script module lang="ts">
  /** The reference's Power submenu, in its order. */
  export const POWER_ITEMS = ['Restart AIONE', 'Close AIONE', 'Sleep Mode', 'Restart PC', 'Turn Off PC'] as const;
</script>

<script lang="ts">
  // B-26 and the reference's Quick Menu: opens with a held Guide, slides in from the right, flush
  // with the header and footer; the whole screen (header included) dims under it (no band, L-13).
  // It owns input while open (one interaction context; App routes to it) and restores Home's focus
  // on close. Rows: Volume (AIONE's own: left / right, LT / RT, A mutes), Theme, Power (a submenu).
  import { fade, fly } from 'svelte/transition';
  import Footer from '../frame/Footer.svelte';
  import type { GlyphDevice, Prompt } from '../frame/glyphs';

  let {
    row,
    power,
    volume,
    muted,
    theme,
    device,
    top,
    bottom,
    onRow,
    onPower,
    onPrompt,
  }: {
    row: number;
    power: number | null;
    volume: number;
    muted: boolean;
    theme: string;
    device: GlyphDevice;
    top: number;
    bottom: number;
    onRow: (row: number) => void;
    onPower: (item: number) => void;
    onPrompt: (button: Prompt['button']) => void;
  } = $props();

  const prompts = $derived<Prompt[]>(
    power === null && row === 0
      ? [{ button: 'back', label: 'Back' }, { button: 'confirm', label: muted ? 'Unmute' : 'Mute' }]
      : [{ button: 'back', label: 'Back' }, { button: 'confirm', label: 'Select' }],
  );
</script>

<div class="qm-scrim" data-testid="qm-scrim" transition:fade={{ duration: 180 }}></div>
<div
  class="qm"
  role="menu"
  aria-label="Quick Menu"
  data-testid="quick-menu"
  style:top={`${top}px`}
  style:bottom={`${bottom}px`}
  transition:fly={{ x: 480, duration: 220, opacity: 1 }}
>
  {#if power === null}
    <div class="row volume focusable" class:focused={row === 0} data-testid="qm-volume" role="menuitem" tabindex="-1" onclick={() => onRow(0)} onkeydown={() => {}}>
      <svg class="icon" viewBox="0 0 24 24" aria-hidden="true">
        <path d="M4 9v6h4l5 4V5L8 9H4z" />
        {#if muted}<path d="M16 9l5 6M21 9l-5 6" class="stroke" />{:else}<path d="M16.5 8.5a5 5 0 0 1 0 7M19 6a8.5 8.5 0 0 1 0 12" class="stroke" />{/if}
      </svg>
      <div class="slider" aria-label="Volume" data-testid="qm-volume-value" data-value={volume} data-muted={muted}>
        <div class="fill" style:width={`${muted ? 0 : volume}%`}></div>
      </div>
    </div>
    <div class="row focusable" class:focused={row === 1} data-testid="qm-theme" role="menuitem" tabindex="-1" onclick={() => onRow(1)} onkeydown={() => {}}>
      <span class="label">Theme</span><span class="value">{theme}</span>
    </div>
    <div class="row focusable" class:focused={row === 2} data-testid="qm-power" role="menuitem" tabindex="-1" onclick={() => onRow(2)} onkeydown={() => {}}>
      <svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 3v9" class="stroke" /><path d="M6.3 6.8a8 8 0 1 0 11.4 0" class="stroke" /></svg>
      <span class="label">Power</span>
    </div>
  {:else}
    <h2 class="title">Power</h2>
    {#each POWER_ITEMS as item, i (item)}
      <div class="row focusable" class:focused={power === i} data-testid="qm-power-item" role="menuitem" tabindex="-1" onclick={() => onPower(i)} onkeydown={() => {}}>
        <span class="label">{item}</span>
      </div>
    {/each}
  {/if}
</div>
<div class="qm-footer" data-testid="qm-footer"><Footer {prompts} {device} {onPrompt} /></div>

<style>
  .qm-scrim {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.55);
    z-index: 900;
  }
  .qm {
    position: fixed;
    right: 0;
    width: calc(520 * var(--px, 1px));
    background: var(--color-surface, #ebebeb);
    color: var(--color-text, #2d2d2d);
    z-index: 910;
    padding: calc(24 * var(--px, 1px)) 0;
    display: flex;
    flex-direction: column;
    gap: calc(6 * var(--px, 1px));
    box-shadow: calc(-4 * var(--px, 1px)) 0 calc(16 * var(--px, 1px)) rgb(0 0 0 / 0.18);
  }
  .title {
    margin: 0 calc(32 * var(--px, 1px)) calc(10 * var(--px, 1px));
    font: 500 calc(31 * var(--px, 1px)) / 1.2 var(--font-ui);
  }
  .row {
    display: flex;
    align-items: center;
    gap: calc(18 * var(--px, 1px));
    margin: 0 calc(16 * var(--px, 1px));
    padding: calc(18 * var(--px, 1px)) calc(16 * var(--px, 1px));
    font: 400 calc(29 * var(--px, 1px)) / 1.2 var(--font-ui);
    border-radius: calc(6 * var(--px, 1px));
    cursor: pointer;
  }
  /* The shared focus look (A6): the theme's accent, as on the dialog buttons. */
  .row.focused {
    box-shadow: inset 0 0 0 calc(4 * var(--px, 1px)) var(--color-accent, #fd007e);
    background: var(--color-surface-raised, #f6f6f6);
  }
  .value {
    margin-left: auto;
    opacity: 0.75;
  }
  .icon {
    width: calc(36 * var(--px, 1px));
    height: calc(36 * var(--px, 1px));
    flex: none;
    fill: currentColor;
  }
  .icon .stroke {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
  }
  .slider {
    flex: 1;
    height: calc(8 * var(--px, 1px));
    border-radius: calc(4 * var(--px, 1px));
    background: rgb(128 128 128 / 0.35);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--color-accent, #00c3e3);
  }
  .qm-footer {
    position: fixed;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 920;
  }
</style>
