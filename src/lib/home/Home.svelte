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
  import { titleAlign, titleIndex, type HomeState } from './model';

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
    mediaUrl: (g: GameView) => string;
    fixedTime?: Date | null;
    hour12?: boolean;
    onHover: (index: number | null) => void;
    onClick: (index: number) => void;
  } = $props();

  let broken = $state(new Set<string>());
  const shownTitle = $derived(suspended ? null : titleIndex(home, mode));
</script>

<section class="home" data-testid="home" data-owner="home" data-suspended={suspended} data-mode={mode}>
  <StatusHeader {padConnected} {status} {fixedTime} {...hour12 === undefined ? {} : { hour12 }} />

  <div class="row" data-testid="game-row" data-first={home.first} data-focus={home.focus} data-count={games.length}>
    {#if loadState === 'loaded'}
      <div class="track" style:--first={home.first} data-owner="home-row">
        {#each games as g, i (g.id)}
          {@const url = g.tileArt && !broken.has(g.id) ? mediaUrl(g) : ''}
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
                  alt=""
                  draggable="false"
                  decoding="async"
                  onerror={() => (broken = new Set([...broken, g.id]))}
                />
              {:else}
                <span class="plain" data-testid="tile-plain">{g.name}</span>
              {/if}
            </div>
            <span class="hover-line" aria-hidden="true"></span>
            <span class="ring" aria-hidden="true" data-owner="home-ring"><span class="ring-line"></span></span>
            {#if shownTitle === i}
              <div class="title align-{titleAlign(home, i)}" data-testid="selected-title" data-owner="home-title">
                <Marquee text={g.name} align={titleAlign(home, i)} testid="selected-title-text" />
              </div>
            {/if}
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
  .tile.focused .ring-line {
    animation: ring-blink var(--motion-blink) ease-in-out var(--motion-focus) infinite;
  }
  @keyframes ring-blink {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.45;
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

  .title {
    position: absolute;
    bottom: calc(100% + 0.75rem);
    width: 150%;
    color: var(--color-title);
    font-size: var(--text-title);
    font-weight: 400;
    line-height: 1.35;
    letter-spacing: 0.01em;
  }
  .align-start {
    left: 0;
  }
  .align-center {
    left: -25%;
  }
  .align-end {
    right: 0;
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
