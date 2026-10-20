<script module lang="ts">
  /** Rev. ax: Power options, Switch's order (a short centered list, PowerOptions.svelte). */
  export const POWER_ITEMS = ['Sleep Mode', 'Restart PC', 'Turn Off PC', 'Restart AIONE', 'Close AIONE'] as const;
  /** Rev. ax: the Quick Menu's rows, in this order. */
  export const QM_ROWS = ['Power', 'Brightness', 'Volume', 'Theme'] as const;
</script>

<script lang="ts">
  // B-26 (rev. ax; the user's Switch Quick Settings photo): a full-height panel from the right
  // edge, about 40 % wide. The status items sit on its top and its own footer at its bottom, so
  // the header and footer flow into it; its colours follow the preset; the rest is dimmed. Rows
  // separated by thin lines; sliders with their icon; a short line under a setting where useful;
  // the B3 ring on the focused row. It owns input while open (App routes to it).
  import { fade, fly } from 'svelte/transition';
  import Footer from '../frame/Footer.svelte';
  import StatusHeader from '../frame/StatusHeader.svelte';
  import Slider from './Slider.svelte';
  import type { GlyphDevice, Prompt } from '../frame/glyphs';
  import type { PcStatus } from '../core/types';

  let {
    row,
    brightness,
    volume,
    muted,
    theme,
    device,
    padConnected,
    status,
    fixedTime = null,
    glide = true,
    hour12 = undefined,
    onRow,
    onPrompt,
  }: {
    row: number;
    brightness: { method: string; value: number | null; reason: string | null };
    volume: number;
    muted: boolean;
    theme: string;
    device: GlyphDevice;
    padConnected: boolean;
    status: PcStatus | null;
    fixedTime?: Date | null;
    /** Rev. ay: false while a held direction drives a slider frame by frame. */
    glide?: boolean;
    hour12?: boolean;
    onRow: (row: number) => void;
    onPrompt: (button: Prompt['button']) => void;
  } = $props();

  const prompts: Prompt[] = [
    { button: 'back', label: 'Back' },
    { button: 'confirm', label: 'OK' },
  ];
  const canBrightness = $derived(brightness.method !== 'none' && brightness.value !== null);
</script>

<div class="qm-scrim" data-testid="qm-scrim" transition:fade={{ duration: 180 }}></div>
<div class="qm" role="menu" aria-label="Quick Menu" data-testid="quick-menu" transition:fly={{ x: 600, duration: 220, opacity: 1 }}>
  <StatusHeader {padConnected} {status} {fixedTime} {...hour12 === undefined ? {} : { hour12 }} />
  <div class="rows">
    {#each QM_ROWS as name, i (name)}
      <div class="item" class:focused={row === i} data-testid={`qm-${name.toLowerCase()}`} role="menuitem" tabindex="-1" onclick={() => onRow(i)} onkeydown={() => {}}>
        {#if name === 'Power'}
          <svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 3v9" class="stroke" /><path d="M6.3 6.8a8 8 0 1 0 11.4 0" class="stroke" /></svg>
          <span class="label">Power</span>
        {:else if name === 'Brightness'}
          <svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="4" /><path d="M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9l2.1 2.1M17 17l2.1 2.1M4.9 19.1 7 17M17 7l2.1-2.1" class="stroke" /></svg>
          {#if canBrightness}
            <Slider value={brightness.value ?? 0} {glide} testid="qm-brightness-value" label="Brightness" />
          {:else}
            <span class="note" data-testid="qm-brightness-note">{brightness.reason ?? "Brightness can't be changed for this display"}</span>
          {/if}
        {:else if name === 'Volume'}
          <svg class="icon" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M4 9v6h4l5 4V5L8 9H4z" />
            {#if muted}<path d="M16 9l5 6M21 9l-5 6" class="stroke" />{:else}<path d="M16.5 8.5a5 5 0 0 1 0 7M19 6a8.5 8.5 0 0 1 0 12" class="stroke" />{/if}
          </svg>
          <span class="mute-state" data-testid="qm-volume-muted" data-muted={muted} hidden></span>
          <Slider value={muted ? 0 : volume} {glide} testid="qm-volume-value" label="Volume" />
        {:else}
          <span class="label">Theme</span><span class="value">{theme}</span>
        {/if}
        <span class="ring" aria-hidden="true"><span class="ring-halo"></span><span class="ring-line"></span><span class="ring-line ring-alt"></span></span>
      </div>
      {#if name === 'Volume' && row === i}
        <p class="explain" data-testid="qm-explain">AIONE's own volume; Windows' volume isn't changed. A mutes.</p>
      {/if}
    {/each}
  </div>
  <Footer {prompts} {device} {onPrompt} />
</div>

<style>
  .qm-scrim {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.5);
    z-index: 900;
  }
  .qm {
    position: fixed;
    top: 0;
    right: 0;
    bottom: 0;
    width: 40vw;
    background: var(--color-surface, #ffffff);
    color: var(--color-text, #2d2d2d);
    z-index: 910;
  }
  .rows {
    position: absolute;
    left: 0;
    right: 0;
    top: calc(170 * var(--px, 1px));
    padding: 0 calc(36 * var(--px, 1px));
  }
  .item {
    position: relative;
    display: flex;
    align-items: center;
    gap: calc(22 * var(--px, 1px));
    min-height: calc(84 * var(--px, 1px));
    padding: 0 calc(22 * var(--px, 1px));
    border-bottom: calc(1 * var(--px, 1px)) solid rgb(128 128 128 / 0.35);
    /* Rev. ay: the footer's text size and weight, so at 100 % it matches the header and footer. */
    font: 400 var(--text-label) / 1.2 var(--font-ui);
    cursor: pointer;
  }
  .item:first-child {
    border-top: calc(1 * var(--px, 1px)) solid rgb(128 128 128 / 0.35);
  }
  .value {
    margin-left: auto;
    color: var(--color-accent, #fd007e);
  }
  .note,
  .explain {
    font-size: calc(var(--text-label) * 0.85);
    color: var(--color-text-muted, #5c5c5c);
  }
  .explain {
    margin: calc(10 * var(--px, 1px)) calc(22 * var(--px, 1px)) calc(6 * var(--px, 1px));
  }
  .icon {
    width: calc(38 * var(--px, 1px));
    height: calc(38 * var(--px, 1px));
    flex: none;
    fill: currentColor;
  }
  .icon .stroke {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
  }
  /* The B3 ring (the same markup and flash as Home's; app.css holds the keyframes). */
  .ring {
    position: absolute;
    inset: calc(-4 * var(--px, 1px));
    border-radius: calc(8 * var(--px, 1px));
    opacity: 0;
    pointer-events: none;
  }
  .ring-line {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    border: calc(5 * var(--px, 1px)) solid var(--color-ring);
    box-shadow: inset 0 0 0 calc(1 * var(--px, 1px)) var(--color-ring-edge);
  }
  .ring-halo {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    box-shadow: 0 0 calc(4 * var(--px, 1px)) 0 var(--color-ring-peak);
    opacity: 0;
  }
  .ring-alt {
    border-color: var(--color-ring-peak);
    box-shadow: none;
    opacity: 0;
  }
  .item.focused .ring {
    opacity: 1;
  }
  .item.focused .ring-alt,
  .item.focused .ring-halo {
    animation: aione-ring-flash 1000ms linear infinite;
  }
</style>
