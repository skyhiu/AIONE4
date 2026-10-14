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
  import { DRAWER, titleIndex, type HomeState } from './model';

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
    onDrawerHover = () => {},
    onDrawerClick = () => {},
    drawerPressed = null,
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
    /** T1.2: the drawer (mouse). */
    onDrawerHover?: (index: number | null) => void;
    onDrawerClick?: (index: number) => void;
    /** The drawer button showing its press (squash-and-zoom), or null. */
    drawerPressed?: number | null;
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
      <!-- B-8 (rev. o): just like Switch, the title alone in the accent text colour, above the
           selected tile from its left edge; moved by transform only; shifted left near the right
           margin; a long title scrolls within its own space, clipped at both ends. -->
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
            class:focused={!suspended && mode === 'pad' && home.zone === 'row' && home.focus === i}
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
              ><span class="ring-line"></span><span class="ring-line ring-alt"></span></span
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
  {#if loadState === 'loaded'}
    <!-- T1.2: the System Drawer (reference Part B; Switch-style in the palette, B-9 rev. o). -->
    <nav class="drawer" data-testid="drawer" data-zone={home.zone} data-focus={home.drawer} aria-label="System">
      {#each DRAWER as item, i (item.id)}
        {@const focused = mode === 'pad' && !suspended && home.zone === 'drawer' && home.drawer === i}
        {@const hovered = mode === 'mouse' && !suspended && home.drawerHover === i}
        <div class="ditem">
          <button
            type="button"
            class="dbtn"
            class:focused
            class:hovered
            class:pressed={drawerPressed === i}
            data-testid="drawer-button"
            data-id={item.id}
            data-index={i}
            aria-label={item.label}
            tabindex="-1"
            onmouseenter={() => onDrawerHover(i)}
            onmouseleave={() => onDrawerHover(null)}
            onclick={() => onDrawerClick(i)}
          >
            <span class="icon dicon" style:--w={item.weight} aria-hidden="true">{item.icon}</span>
            <span class="dring" aria-hidden="true" data-owner="drawer-ring"
              ><span class="dring-line"></span><span class="dring-line dring-alt"></span></span
            >
          </button>
          <!-- The label sits under the button on the page's own background (not the button's). -->
          {#if focused || hovered}
            <span class="dlabel" data-testid="drawer-label">{item.label}</span>
          {/if}
        </div>
      {/each}
    </nav>
  {/if}
  <Footer
    prompts={loadState === 'loaded' && !suspended ? [{ button: 'confirm', label: home.zone === 'drawer' && mode === 'pad' ? 'OK' : 'Start' }] : []}
    {device}
    branding
  />
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
  /* B-9 (rev. m): like Switch's selection frame, the ring itself pulses smoothly between two
     shades of the accent, with no glow. Two solid rings in the two shades; only the top one's
     opacity changes (A7: opacity only), so every moment is a blend of two colours that both meet
     3:1 (D-011 stays met). 1.4 s per cycle, eased, starting after the focus-in. */
  .ring-alt {
    border-color: var(--color-ring-alt);
    opacity: 0;
  }
  .tile.focused .ring-alt {
    animation: ring-pulse calc(var(--motion-blink) / 2) ease-in-out var(--motion-focus) infinite alternate;
  }
  @keyframes ring-pulse {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
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
  .title-text {
    flex: 0 1 auto;
    min-width: 0;
    overflow: hidden;
  }

  /* ---- The drawer (T1.2). Design px at 1080 p, scaled with Interface Scale (--ui): buttons
     108 across, 28 apart, centred (Switch's proportions: disc / tile 0.28, measured on the
     reference image); 86 px under the game row where it fits; the label 12 under the button. ---- */
  .drawer {
    position: absolute;
    left: 0;
    right: 0;
    /* Like Switch: 86 px under the row (at 1080 p); but never so low that the label (12 under the
       button, its line 1.75 rem × 1.15) runs into the footer (5.5 rem) at large Interface Scales:
       then it moves up just enough, keeping 12 px clear. */
    top: min(
      calc((292 + 384 + 86) * var(--px)),
      calc(100vh - 5.5rem - 120 * var(--ui) - 2.0125rem - 12 * var(--px))
    );
    display: flex;
    justify-content: center;
    gap: calc(28 * var(--ui));
  }
  .ditem {
    position: relative;
  }
  .dbtn {
    position: relative;
    width: calc(108 * var(--ui));
    height: calc(108 * var(--ui));
    border-radius: 50%;
    border: none;
    padding: 0;
    background: var(--color-surface);
    color: var(--color-text);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: default;
    outline: none;
  }
  .dicon {
    font-size: calc(48 * var(--ui));
    line-height: 1;
    font-variation-settings:
      'wght' var(--w, 300),
      'FILL' 0,
      'GRAD' 0,
      'opsz' 48;
    transition: transform var(--motion-press) var(--ease-out);
  }
  /* Press: the icon's squash-and-zoom (reference Part B), never delaying the action. */
  .dbtn.pressed .dicon {
    animation: squash 260ms var(--ease-out);
  }
  @keyframes squash {
    0% {
      transform: scale(1);
    }
    35% {
      transform: scale(0.84);
    }
    70% {
      transform: scale(1.1);
    }
    100% {
      transform: scale(1);
    }
  }
  /* The focus ring: like the tiles' (B-9 rev. m): two shades of the accent, the top one's
     opacity pulsing; 6 px with a 5 px gap, so 11 px outside the button. */
  .dring {
    position: absolute;
    inset: calc(-11 * var(--ui));
    border-radius: 50%;
    opacity: 0;
    pointer-events: none;
  }
  .dring-line {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    border: calc(6 * var(--ui)) solid var(--color-ring);
  }
  .dring-alt {
    border-color: var(--color-ring-alt);
    opacity: 0;
  }
  .dbtn.focused .dring {
    opacity: 1;
  }
  .dbtn.focused .dring-alt {
    animation: ring-pulse calc(var(--motion-blink) / 2) ease-in-out var(--motion-focus) infinite alternate;
  }
  .dbtn.hovered {
    box-shadow: 0 0 0 calc(4 * var(--ui)) var(--color-hover);
  }
  /* The label under the focused button, in the accent text colour, centred, never clipped. */
  .dlabel {
    position: absolute;
    top: calc(100% + 12 * var(--ui));
    left: 50%;
    transform: translateX(-50%);
    white-space: nowrap;
    color: var(--color-title);
    font-size: var(--text-label);
    font-weight: 500;
    line-height: 1.15;
    pointer-events: none;
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
