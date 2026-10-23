<script lang="ts">
  // B-12 (reference A7): AIONE's own right-stick pointer (never the Windows cursor). A16: the
  // Material Symbols Rounded pointer arrow (`arrow_selector_tool`, filled), in the focus ring's
  // pink (B-9: --color-ring, #D9006C) with a white edge (the ring's white band, --color-ring-gap), so it reads
  // over any art; 64 px (alpha.37 visual check: 48 px read small), following Interface Scale. Moved by transform only (A7); never takes a
  // click (pointer-events: none).
  interface Props {
    x: number;
    y: number;
    shown: boolean;
  }
  let { x, y, shown }: Props = $props();
</script>

<span class="pointer" class:shown data-testid="pointer" data-shown={shown} aria-hidden="true" style:transform={`translate(${x}px, ${y}px)`}
  ><span class="icon edge">arrow_selector_tool</span><span class="icon fill">arrow_selector_tool</span></span
>

<style>
  /* alpha.37 visual check (round 2): -webkit-text-stroke doesn't draw on this icon font, so the
     white edge is the same glyph twice: white at weight 700 behind the pink at weight 300 (the
     heavier glyph is slightly larger all round). No filter or shadow (A7). */
  .pointer {
    position: fixed;
    left: 0;
    top: 0;
    z-index: 2000;
    /* The arrow's tip is at the glyph's top-left quarter: shift so the tip is the point. */
    margin: calc(-16 * var(--ui)) 0 0 calc(-19 * var(--ui));
    width: calc(64 * var(--ui));
    height: calc(64 * var(--ui));
    pointer-events: none;
    opacity: 0;
    transition: opacity var(--motion-fast) var(--ease-out);
    will-change: transform;
  }
  .pointer.shown {
    opacity: 1;
  }
  .pointer .icon {
    position: absolute;
    inset: 0;
    font-size: calc(64 * var(--ui));
    line-height: 1;
  }
  .edge {
    color: var(--color-ring-gap);
    font-variation-settings:
      'wght' 700,
      'FILL' 1,
      'GRAD' 0,
      'opsz' 48;
  }
  .fill {
    color: var(--color-ring);
    font-variation-settings:
      'wght' 300,
      'FILL' 1,
      'GRAD' 0,
      'opsz' 48;
  }
</style>
