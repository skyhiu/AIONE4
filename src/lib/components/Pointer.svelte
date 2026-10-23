<script lang="ts">
  // B-12 (reference A7): AIONE's own right-stick pointer (never the Windows cursor). A16: the
  // Material Symbols Rounded pointer arrow (`arrow_selector_tool`, filled), in the focus ring's
  // pink (B-9: --color-ring, #D9006C) with a white edge (the ring's white band, --color-ring-gap), so it reads
  // over any art; 48 px, following Interface Scale. Moved by transform only (A7); never takes a
  // click (pointer-events: none).
  interface Props {
    x: number;
    y: number;
    shown: boolean;
  }
  let { x, y, shown }: Props = $props();
</script>

<span
  class="pointer icon"
  class:shown
  data-testid="pointer"
  data-shown={shown}
  aria-hidden="true"
  style:transform={`translate(${x}px, ${y}px)`}>arrow_selector_tool</span
>

<style>
  .pointer {
    position: fixed;
    left: 0;
    top: 0;
    z-index: 2000;
    /* The arrow's tip is at the glyph's top-left quarter: shift so the tip is the point. */
    margin: calc(-12 * var(--ui)) 0 0 calc(-14 * var(--ui));
    font-size: calc(48 * var(--ui));
    color: var(--color-ring);
    -webkit-text-stroke: calc(2.5 * var(--ui)) var(--color-ring-gap);
    paint-order: stroke fill;
    font-variation-settings:
      'wght' 400,
      'FILL' 1,
      'GRAD' 0,
      'opsz' 48;
    pointer-events: none;
    opacity: 0;
    transition: opacity var(--motion-fast) var(--ease-out);
    will-change: transform;
  }
  .pointer.shown {
    opacity: 1;
  }
</style>
