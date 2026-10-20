<script lang="ts">
  // Rev. ax: Power options, a short centered list like Switch's (supersedes alpha.31's second side
  // panel). Opened from the Quick Menu's Power row and the drawer's Power button. Destructive
  // choices ask first (App). The B3 ring on the focused item; B closes it.
  import { fade, scale } from 'svelte/transition';
  import { POWER_ITEMS } from './QuickMenu.svelte';

  let { focus, onPick }: { focus: number; onPick: (i: number) => void } = $props();
</script>

<div class="po-scrim" transition:fade={{ duration: 160 }}></div>
<div class="po" role="menu" aria-label="Power Options" data-testid="power-options" transition:scale={{ duration: 200, start: 0.96 }}>
  <h1>Power Options</h1>
  {#each POWER_ITEMS as item, i (item)}
    <div class="item" class:focused={focus === i} data-testid="po-item" role="menuitem" tabindex="-1" onclick={() => onPick(i)} onkeydown={() => {}}>
      {item}
      <span class="ring" aria-hidden="true"><span class="ring-halo"></span><span class="ring-line"></span><span class="ring-line ring-alt"></span></span>
    </div>
  {/each}
</div>

<style>
  .po-scrim {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.55);
    z-index: 940;
  }
  .po {
    position: fixed;
    left: 50%;
    top: 50%;
    translate: -50% -50%;
    width: calc(620 * var(--px, 1px));
    padding: calc(28 * var(--px, 1px)) calc(28 * var(--px, 1px)) calc(20 * var(--px, 1px));
    border-radius: calc(12 * var(--px, 1px));
    background: var(--color-surface, #fff);
    color: var(--color-text, #2d2d2d);
    z-index: 945;
    box-shadow: 0 calc(8 * var(--px, 1px)) calc(28 * var(--px, 1px)) rgb(0 0 0 / 0.25);
  }
  h1 {
    margin: 0 0 calc(16 * var(--px, 1px));
    font: 500 calc(32 * var(--px, 1px)) / 1.2 var(--font-ui);
    text-align: center;
  }
  .item {
    position: relative;
    padding: calc(20 * var(--px, 1px)) calc(22 * var(--px, 1px));
    border-top: calc(1 * var(--px, 1px)) solid rgb(128 128 128 / 0.35);
    font: 400 calc(30 * var(--px, 1px)) / 1.2 var(--font-ui);
    cursor: pointer;
  }
  .ring {
    position: absolute;
    inset: calc(-2 * var(--px, 1px));
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
