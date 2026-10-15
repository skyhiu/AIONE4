<script lang="ts">
  // The shared footer (reference A4 / A7): a thin divider, the branding at the left (Home only)
  // and the button prompts at the right, each a Kenney glyph plus a short label, bolder and
  // larger than body text, on one baseline, scaling with Interface Scale.
  import { GLYPHS, type GlyphDevice, type Prompt } from './glyphs';

  let {
    prompts,
    device,
    branding = false,
    onPrompt,
  }: { prompts: Prompt[]; device: GlyphDevice; branding?: boolean; onPrompt?: ((button: Prompt['button']) => void) | undefined } = $props();
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
        <!-- B-16: the prompts are clickable (a click acts like the button). -->
        <span
          class="prompt"
          class:dimmed={p.dimmed}
          data-testid="prompt-{p.button}"
          data-dimmed={p.dimmed === true}
          role="button"
          tabindex="-1"
          onclick={() => onPrompt?.(p.button)}
          onkeydown={() => {}}
        >
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
    /* rev. aa: about 20 px higher (the divider at about y 972 at 1080 p / 100 %, like Switch's).
       The extra height shrinks to nothing at 150 % Interface Scale, where the drawer needs the
       room (alpha.15's first build overlapped the row by 11 px there). */
    height: calc(5.5rem + 20 * var(--px) * max(0, 3 - 2 * var(--scale)));
    /* rev. y: a 2 px divider, a darker grey than alpha.13's faint line, as on Switch. */
    border-top: max(2px, calc(2 * var(--px))) solid var(--color-footer-divider);
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
    /* rev. y: about 63 px of background between prompts, measured ink to ink (the glyph's art has
       about 7 px of padding in its box, D-023). */
    gap: calc(53 * var(--ui));
  }
  /* B-14: a dimmed prompt in the muted text colour (still ≥ 4.5:1, A8). */
  .prompt.dimmed {
    color: var(--color-text-muted);
  }
  .prompt {
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: calc(7 * var(--ui)); /* rev. y: about 16 px ink to ink (measured: the art adds about 9) */
  }
  .glyph {
    /* rev. y / D-023: the VISIBLE circle about 37 px, measured at 50 % coverage (Kenney's circle
       fills about 75 % of its box; alpha.14's first 51 px box gave 38.5 px, CI run #21), so the box
       is 49 px. */
    width: 3.075rem;
    height: 3.075rem;
    background: currentColor;
    -webkit-mask: var(--glyph) center / contain no-repeat;
    mask: var(--glyph) center / contain no-repeat;
  }
  .label {
    font-size: var(--text-label);
    font-weight: 400; /* rev. y: regular */
    line-height: 1;
  }
</style>
