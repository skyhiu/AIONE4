<script lang="ts">
  // The shared footer (reference A4 / A7): a thin divider, the branding at the left (Home only)
  // and the button prompts at the right, each a Kenney glyph plus a short label, bolder and
  // larger than body text, on one baseline, scaling with Interface Scale.
  import { GLYPHS, type GlyphDevice, type Prompt } from './glyphs';
  import markUrl from '../../../assets/brand/aione_mark_white.svg?url';

  let {
    prompts,
    device,
    branding = false,
  }: { prompts: Prompt[]; device: GlyphDevice; branding?: boolean } = $props();
</script>

<footer class="footer" data-testid="footer" data-owner="footer">
  <div class="brand" data-testid="footer-branding">
    {#if branding}
      <!-- B-5 final: the one-colour "Window B" mark + "AIONE". Drawn in the footer's text colour
           (off-white on dark; dark on the light preset, where off-white would be invisible). -->
      <span class="mark" data-testid="footer-mark" style:--mark={`url(${JSON.stringify(markUrl)})`} aria-hidden="true"></span>
      <span class="name" data-testid="system-name">AIONE</span>
    {/if}
  </div>
  <div class="prompts" data-testid="footer-prompts" data-device={device}>
    {#each prompts as p (p.button + p.label)}
      <span class="prompt" data-testid="prompt-{p.button}">
        <span
          class="glyph"
          data-glyph={GLYPHS[device][p.button].file}
          style:--glyph={`url(${JSON.stringify(GLYPHS[device][p.button].url)})`}
          aria-hidden="true"
        ></span>
        <span class="label">{p.label}</span>
      </span>
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
    gap: 0.6rem;
  }
  /* 22 design px at 100 % (16–24 px, B-5 final). */
  .mark {
    width: 1.375rem;
    height: 1.375rem;
    background: currentColor;
    -webkit-mask: var(--mark) center / contain no-repeat;
    mask: var(--mark) center / contain no-repeat;
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
