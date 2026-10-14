<script lang="ts">
  // Switch-style Home, still (T1.1): the status header, the game row with its focus ring, the
  // selected game's title, and the footer with prompts and branding. No drawer, background
  // video or launching yet (T1.2, T1.3); the drawer's space below the row is kept free so the
  // layout does not move when it arrives.
  //
  // Input arrives as actions from App.svelte (one owner of input, A6); the mouse acts on the
  // tiles directly. While a dialog is open, Home is suspended: no ring, blink, title, marquee or
  // hover (A6 / L-18).
  import type { GameView, PcStatus } from '../core/types';
  import StatusHeader from '../frame/StatusHeader.svelte';
  import Footer from '../frame/Footer.svelte';
  import Marquee from '../frame/Marquee.svelte';
  import type { GlyphDevice } from '../frame/glyphs';
  import { titleIndex, type HomeState } from './model';

  let {
    games,
    state: home,
    mode,
    suspended,
    loadState,
    device,
    padConnected,
    status,
    pressed,
    mediaUrl,
    onPicture = () => {},
    fixedTime = null,
    hour12,
    onHover,
    onClick,
  }: {
    games: GameView[];
    state: HomeState;
    mode: 'pad' | 'mouse';
    suspended: boolean;
    loadState: 'loading' | 'loaded' | 'empty' | 'error';
    device: GlyphDevice;
    padConnected: boolean;
    status: PcStatus | null;
    pressed: number | null;
    /** 'tile' = the core's copy; any other slot = the original picture (until the copy exists). */
    mediaUrl: (g: GameView, slot: string) => string;
    onPicture?: (what: string, width: number, height: number) => void;
    fixedTime?: Date | null;
    hour12?: boolean;
    onHover: (index: number | null) => void;
    onClick: (index: number) => void;
  } = $props();

  let broken = $state(new Set<string>());
  let vw = $state(1920);
  let vh = $state(1080);
  let titleEl: HTMLDivElement | undefined = $state();
  /** The title box's own width, measured once per title / size change (never in a loop). */
  let titleW = $state(0);
  /** No transition for the very first placement (it would slide in from the left). */
  let placed = $state(false);
  const shownTitle = $derived(suspended ? null : titleIndex(home, mode));
  // Geometry in CSS px (design px × the window's height / 1080; see the design system).
  const px = $derived(vh / 1080);
  const rowStart = $derived(160 * px);
  const rightEdge = $derived(vw * 0.95);
  const maxTitleW = $derived(Math.max(0, rightEdge - rowStart));
  const tileLeft = $derived(shownTitle === null ? rowStart : rowStart + (shownTitle - home.first) * 404 * px);
  /** B-8: left-aligned with the tile; shifted left just enough to stay inside the margin. */
  const titleX = $derived(Math.max(rowStart, Math.min(tileLeft, rightEdge - titleW)));
  const shifted = $derived(titleX < tileLeft - 0.5);

  $effect(() => {
    // Re-measure when the title, the window or the scale changes (one read, no loop: L-16).
    void shownTitle;
    void vw;
    void vh;
    const el = titleEl;
    if (!el) return;
    titleW = el.getBoundingClientRect().width;
    if (!placed) requestAnimationFrame(() => (placed = true));
  });
</script>

<svelte:window bind:innerWidth={vw} bind:innerHeight={vh} />

<section class="home" data-testid="home" data-owner="home" data-suspended={suspended} data-mode={mode}>
  <StatusHeader {padConnected} {status} {fixedTime} {...hour12 === undefined ? {} : { hour12 }} />

  <div class="row" data-testid="game-row" data-first={home.first} data-focus={home.focus} data-count={games.length}>
    {#if loadState === 'loaded' && shownTitle !== null && games[shownTitle]}
      {@const tg = games[shownTitle]}
      <!-- B-8 (rev. k / l): above the selected tile, left-aligned with it, after a downward arrow;
           moved by transform only; shifted left near the right margin; long titles scroll. -->
      <div
        class="title"
        class:placed
        bind:this={titleEl}
        style:transform={`translateX(${titleX}px)`}
        style:max-width={`${maxTitleW}px`}
        data-testid="selected-title-row"
        data-index={shownTitle}
        data-shifted={shifted}
        data-owner="home-title"
      >
        <span class="arrow" aria-hidden="true"><span class="icon glyph" data-testid="title-arrow">arrow_drop_down</span></span>
        <span class="title-text" data-testid="selected-title"
          ><Marquee text={tg.name} align="start" testid="selected-title-text" /></span
        >
      </div>
    {/if}
    {#if loadState === 'loaded'}
      <div class="track" style:--first={home.first} data-owner="home-row">
        {#each games as g, i (g.id)}
          {@const copy = g.tile && !broken.has(g.id)}
          {@const url = copy ? mediaUrl(g, 'tile') : g.tileArt && !broken.has(`${g.id}#o`) ? mediaUrl(g, g.tileArt) : ''}
          <div
            class="tile"
            role="button"
            tabindex="-1"
            aria-label={g.name}
            data-testid="tile"
            data-index={i}
            class:focused={!suspended && mode === 'pad' && home.focus === i}
            class:hovered={!suspended && mode === 'mouse' && (home.hover === i || home.armed === i)}
            class:pressed={pressed === i}
            onmouseenter={() => onHover(i)}
            onmouseleave={() => onHover(null)}
            onclick={() => onClick(i)}
            onkeydown={() => {}}
          >
            <div class="art">
              {#if url}
                <img
                  src={url}
                  class:fit={!copy && g.tileMode === 'fit'}
                  data-source={copy ? 'copy' : 'original'}
                  alt=""
                  draggable="false"
                  decoding="async"
                  onload={(e) => {
                    const im = e.currentTarget as HTMLImageElement;
                    onPicture(`tile "${g.name}" shown`, im.naturalWidth, im.naturalHeight);
                  }}
                  onerror={() => (broken = new Set([...broken, copy ? g.id : `${g.id}#o`]))}
                />
              {:else}
                <span class="plain" data-testid="tile-plain">{g.name}</span>
              {/if}
            </div>
            <span class="hover-line" aria-hidden="true"></span>
            <span class="ring" aria-hidden="true" data-owner="home-ring"
              ><span class="ring-glow"></span><span class="ring-line"></span></span
            >

          </div>
        {/each}
      </div>
    {:else if loadState === 'empty'}
      <div class="message" data-testid="home-empty">
        <p class="headline">No games yet</p>
        <p>Your games will appear here.</p>
      </div>
    {:else if loadState === 'error'}
      <div class="message" data-testid="home-error">
        <p class="headline">Your games could not be loaded</p>
        <p>The details are in aione.log.</p>
      </div>
    {/if}
  </div>

  <!-- While a dialog is open its own prompts apply; Home's are hidden (they would not act). -->
  <Footer prompts={loadState === 'loaded' && !suspended ? [{ button: 'confirm', label: 'Start' }] : []} {device} branding />
</section>

<style>
  .home {
    position: relative;
    height: 100%;
    background: var(--color-bg);
    overflow: hidden;
  }

  /* The row band: the title area above the tiles, then the tiles (row top ≈ 27 % of the
     height, as in the reference). Clipped at the screen edges only. */
  .row {
    position: absolute;
    left: 0;
    right: 0;
    top: calc(292 * var(--px) - 3.75rem);
    height: calc(3.75rem + 384 * var(--px) + 16 * var(--px));
    overflow: clip;
  }
  .track {
    position: absolute;
    left: calc(160 * var(--px));
    top: 3.75rem;
    display: flex;
    gap: calc(20 * var(--px));
    transform: translateX(calc(var(--first) * -404 * var(--px)));
    transition: transform var(--motion-row) var(--ease-out);
    will-change: transform;
  }
  .tile {
    position: relative;
    flex: none;
    width: calc(384 * var(--px));
    height: calc(384 * var(--px));
    outline: none;
    transition: transform var(--motion-release) var(--ease-out);
  }
  .tile.pressed {
    transform: scale(0.97);
    transition-duration: var(--motion-press);
  }
  .art {
    position: absolute;
    inset: 0;
    border-radius: calc(4 * var(--px));
    overflow: hidden;
    background: var(--color-surface);
  }
  .art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  /* B-13, until the core's copy arrives: a non-icon picture shown whole (never cropped). */
  .art img.fit {
    object-fit: contain;
    background: #161616;
  }
  .plain {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: calc(28 * var(--px));
    font-size: calc(30 * var(--px));
    font-weight: 500;
    color: var(--color-text);
    overflow-wrap: anywhere;
  }

  /* Focus ring (pad mode): a teal ring with a gap, fading and settling in while the previous
     one fades out at the same time (A5); then a steady blink of the ring only while idle. */
  .ring {
    position: absolute;
    inset: calc(-11 * var(--px));
    border-radius: calc(10 * var(--px));
    opacity: 0;
    transform: scale(1.025);
    transition:
      opacity var(--motion-focus) var(--ease-in-out),
      transform var(--motion-focus) var(--ease-in-out);
    pointer-events: none;
  }
  .ring-line {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    border: calc(6 * var(--px)) solid var(--color-ring);
    box-shadow: inset 0 0 0 calc(5 * var(--px)) var(--color-ring-gap);
  }
  .tile.focused .ring {
    opacity: 1;
    transform: scale(1);
  }
  /* D-011 (B-9 rev. k): the ring itself never fades; a soft glow pulses outside it. */
  .ring-glow {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    box-shadow: 0 0 calc(18 * var(--px)) calc(5 * var(--px)) var(--color-ring);
    opacity: 0;
  }
  .tile.focused .ring-glow {
    animation: ring-glow var(--motion-blink) ease-in-out var(--motion-focus) infinite;
  }
  @keyframes ring-glow {
    0%,
    100% {
      opacity: 0.15;
    }
    50% {
      opacity: 0.75;
    }
  }

  /* Mouse mode: hover only (and the tile selected by a first click), never the focus ring. */
  .hover-line {
    position: absolute;
    inset: calc(-7 * var(--px));
    border-radius: calc(8 * var(--px));
    border: calc(4 * var(--px)) solid var(--color-hover);
    opacity: 0;
    transition: opacity var(--motion-focus) var(--ease-in-out);
    pointer-events: none;
  }
  .tile.hovered .hover-line {
    opacity: 1;
  }

  /* B-8 (rev. k / l): the title above the selected tile. Positioned at the row's left and moved
     by transform (titleX), with the row's own motion, so it travels with the focus (L-16). */
  .title {
    position: absolute;
    left: 0;
    top: 0;
    height: 3.75rem;
    display: flex;
    align-items: center;
    width: max-content;
    color: var(--color-title);
    font-size: var(--text-title);
    font-weight: 400;
    line-height: 1.35;
    letter-spacing: 0.01em;
    white-space: nowrap;
  }
  .title.placed {
    transition: transform var(--motion-row) var(--ease-out);
  }
  /* The ▼ (Material Symbols Rounded arrow_drop_down). Its visible triangle is 0.19 × 0.365 em of
     its own size, 0.39–0.58 em above its baseline (measured); the title's capitals are 0.73 em.
     So the glyph is set at 0.73 / 0.19 = 3.84 × the title size and placed so that the triangle
     fills this 1.4 × 0.73 em box: cap height, on the title's line. */
  .arrow {
    position: relative;
    flex: none;
    width: 1.4em;
    height: 0.73em;
    margin-right: 0.32em;
    overflow: visible;
    /* Cap-centre of the text sits above the line box's centre; move the box up to it. */
    transform: translateY(var(--arrow-dy, -0.06em));
  }
  .glyph {
    position: absolute;
    font-size: 3.84em;
    line-height: 1;
    /* Glyph box → triangle: left 0.315 em, ink top 0.58 em above the baseline. */
    left: -0.315em;
    /* Measured in the real render (alpha.6): this puts the triangle's top on the capitals' top. */
    top: -0.3915em;
    width: 1em;
    height: 1em;
    color: var(--color-title);
    font-variation-settings:
      'wght' 400,
      'FILL' 1,
      'GRAD' 0,
      'opsz' 24;
  }
  .title-text {
    flex: 0 1 auto;
    min-width: 0;
    overflow: hidden;
  }

  .message {
    position: absolute;
    left: calc(160 * var(--px));
    top: 3.75rem;
    height: calc(384 * var(--px));
    display: flex;
    flex-direction: column;
    justify-content: center;
    color: var(--color-text-muted);
    font-size: var(--text-md);
  }
  .message p {
    margin: 0 0 var(--space-3);
  }
  .headline {
    color: var(--color-text);
    font-size: var(--text-lg);
  }
</style>
