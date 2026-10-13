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
      <!-- The logo joins the name once it is chosen (B-5; proposals in this build). -->
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
