<script lang="ts">
  // The shared footer (reference A4 / A7): a thin divider, the branding at the left (Home only)
  // and the button prompts at the right, each a Kenney glyph plus a short label, bolder and
  // larger than body text, on one baseline, scaling with Interface Scale.
  import { GLYPHS, type GlyphDevice, type Prompt } from './glyphs';

  let {
    prompts,
    device,
    branding = false,
  }: { prompts: Prompt[]; device: GlyphDevice; branding?: boolean } = $props();
</script>

<footer class="footer" data-testid="footer" data-owner="footer">
  <div class="brand" data-testid="footer-branding">
    {#if branding}
      <!-- B-5 (rev. r): the "Window B" mark in one colour (the footer's text colour, no accent),
           about 46 px tall at 1080 p / 100 %, with "AIONE" beside it. -->
      <svg class="mark" data-testid="footer-mark" viewBox="-104 -104 208 208" aria-hidden="true">
        <g transform="rotate(25)">
          <path class="half" d="M -6 -99.82 L -6 -59 L -65 0 L -6 59 L -6 99.82 A 100 100 0 0 1 -6 -99.82 Z" />
          <path class="half" d="M 6 -99.82 L 6 -59 L 65 0 L 6 59 L 6 99.82 A 100 100 0 0 0 6 -99.82 Z" />
          <polygon class="diamond" points="0,-48 48,0 0,48 -48,0" />
        </g>
      </svg>
      <span class="name" data-testid="system-name">AIONE</span>
    {/if}
  </div>
  <div class="prompts" data-testid="footer-prompts" data-device={device}>
    {#each prompts as p (p.button + p.label)}
      {@const glyph = GLYPHS[device][p.button]}
      {#if glyph}
        <span class="prompt" class:dimmed={p.dimmed} data-testid="prompt-{p.button}" data-dimmed={p.dimmed === true}>
          <span class="glyph" data-glyph={glyph.file} style:--glyph={`url(${JSON.stringify(glyph.url)})`} aria-hidden="true"></span>
          <span class="label">{p.label}</span>
        </span>
      {/if}
    {/each}
  </div>
</footer>

<style>
  .footer {
    position: absolute;
    left: 2.4vw;
    right: 2.4vw;
    bottom: 0;
    height: calc(5.5rem);
    border-top: max(1px, calc(1.5 * var(--px))) solid var(--color-divider);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 var(--space-5);
    color: var(--color-text);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }
  /* B-5 (rev. r): 2.875 rem = 46 px at 1080 p and 100 %, scaled with Interface Scale (rem). */
  .mark {
    width: 2.875rem;
    height: 2.875rem;
    flex: none;
    display: block;
  }
  .mark .half {
    fill: currentColor;
  }
  .mark .diamond {
    fill: currentColor;
  }
  .name {
    font-size: var(--text-label);
    font-weight: 500;
    letter-spacing: 0.08em;
  }
  .prompts {
    display: flex;
    align-items: center;
    gap: var(--space-6);
  }
  /* B-14: a dimmed prompt in the muted text colour (still ≥ 4.5:1, A8). */
  .prompt.dimmed {
    color: var(--color-text-muted);
  }
  .prompt {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }
  .glyph {
    width: 2.25rem;
    height: 2.25rem;
    background: currentColor;
    -webkit-mask: var(--glyph) center / contain no-repeat;
    mask: var(--glyph) center / contain no-repeat;
  }
  .label {
    font-size: var(--text-label);
    font-weight: 500;
    line-height: 1;
  }
</style>
