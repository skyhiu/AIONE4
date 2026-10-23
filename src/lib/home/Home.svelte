<script lang="ts">
  import { untrack } from 'svelte';
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
  import type { GlyphDevice } from '../frame/glyphs';
  import { DRAWER, ROW, emptySlots, isEmptySlot, titleIndex, type HomeState } from './model';
  import type { Prompt } from '../frame/glyphs';

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
    onDragTick,
    onDragEnd,
    onPrompt,
    onDrawerHover = () => {},
    onDrawerClick = () => {},
    drawerPressed = null,
    bump = null,
    wrapping = false,
    runningId = null,
    startingId = null,
    scale = 1,
    extras = [],

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
    /** B-16: a tile passed while dragging (the drag tick). */
    onDragTick?: () => void;
    /** B-16: the drag ended: the first tile in view after the glide. */
    onDragEnd?: (first: number) => void;
    /** B-16: a footer prompt was clicked. */
    onPrompt?: (button: Prompt['button']) => void;
    /** T1.2: the drawer (mouse). */
    onDrawerHover?: (index: number | null) => void;
    onDrawerClick?: (index: number) => void;
    /** The drawer button showing its press (squash-and-zoom), or null. */
    drawerPressed?: number | null;
    /** B-14 (rev. q): the latest edge bump (direction, zone and a counter), or null. */
    bump?: { dir: 'left' | 'right' | 'up' | 'down'; n: number; zone: 'row' | 'drawer' } | null;
    /** B-14 (rev. s): the row is whipping to its other end (a 200 ms slide). */
    wrapping?: boolean;
    /** B-18: the running game (its tile shows "Playing"; its footer: Close Software, Continue). */
    runningId?: string | null;
    /** D-051: a start being watched quietly (the tile shows "Starting…"). */
    startingId?: string | null;
    /** Interface Scale as a factor (1 = 100 %), for the drawer's bump distance. */
    scale?: number;
    /** B-15 (rev. t): the extras on this run (all three by default; none with `--extras=none`). */
    extras?: string[];

  } = $props();

  let broken = $state(new Set<string>());
  let vw = $state(1920);
  let vh = $state(1080);
  let titleTrack: HTMLSpanElement | undefined = $state();
  /** The title's natural width, measured once per title / size change (never in a loop). */
  let titleW = $state(0);
  /** No transition for the very first placement (it would slide in from the left). */
  let placed = $state(false);
  const shownTitle = $derived(suspended ? null : titleIndex(home, mode));
  // B-14 geometry in CSS px (design px × the window's height / 1080).
  const px = $derived(vh / 1080);
  const tileLeft = $derived(shownTitle === null ? ROW.left * px : (ROW.left + (shownTitle - home.first) * (ROW.tile + ROW.gap)) * px);
  const tileCentre = $derived(tileLeft + (ROW.tile / 2) * px);
  /** B-14: a long title's scroll window, centred on the tile, 1.8 × the ring's outer width. */
  const windowW = $derived(1.8 * (ROW.tile + 14) * px);
  const longTitle = $derived(titleW > windowW + 0.5);
  const boxW = $derived(Math.min(titleW, windowW));
  /** B-8 (rev. p): centred on the tile; near the screen's edges shifted just enough to stay on
   *  screen (3 % margins). */
  const titleX = $derived(Math.max(vw * 0.03, Math.min(tileCentre - boxW / 2, vw * 0.97 - boxW)));
  const shifted = $derived(Math.abs(titleX - (tileCentre - boxW / 2)) > 0.5);
  const empties = $derived(emptySlots(games.length));
  const onEmpty = $derived(home.zone === 'row' && isEmptySlot(home, home.focus));
  // B-26 / A7: the Home / Guide hint, labeled "Menu", first in the footer wherever Home shows prompts.
  function withMenu(p: Prompt[]): Prompt[] {
    // B-28: the keyboard has its Guide now (Home inside AIONE), so "Menu" shows in both modes.
    return p.length ? [{ button: 'menu', label: 'Menu' }, ...p] : p;
  }
  const basePrompts = $derived<Prompt[]>(
    loadState !== 'loaded' || suspended
      ? []
      : home.zone === 'drawer' && mode === 'pad'
        ? [{ button: 'confirm', label: 'OK' }]
        : onEmpty && mode === 'pad'
          ? [{ button: 'confirm', label: 'OK', dimmed: true }]
          : runningId !== null && games[home.focus]?.id === runningId
            ? // B-18: the running game: Options, X Close Software, A Continue.
              [
                { button: 'options', label: 'Options' } as Prompt,
                { button: 'close', label: 'Close Software' } as Prompt,
                { button: 'confirm', label: 'Continue' },
              ]
            : [
                { button: 'options', label: 'Options' } as Prompt,
                { button: 'confirm', label: 'Start' },
              ],
  );
  const prompts = $derived<Prompt[]>(withMenu(basePrompts));

  // ---------- B-16: dragging the row with the mouse or a finger ----------
  /** The drag in progress: the pointer's start, the row's pitch, the offset so far. */
  let drag: { id: number; x0: number; pitch: number; max: number; moved: boolean; cell: number; samples: [number, number][] } | null = null;
  /** The row's offset while dragging (px), on top of its scroll position. */
  let dragPx = $state(0);
  let dragging = $state(false);
  /** After release: the glide to the snapped tile (about 150 ms, slowing). */
  let gliding = $state(false);
  /** The tile under a press (a pale accent tint) until it drags or lifts. */
  let pressedTile = $state<number | null>(null);
  /** A drag never counts as a click. */
  let suppressClick = false;

  function onRowDown(e: PointerEvent) {
    if (e.button !== 0 || suspended || home.zone !== 'row') return;
    const tiles = document.querySelectorAll<HTMLElement>('.track > .tile');
    const a = tiles[0]?.getBoundingClientRect();
    const b = tiles[1]?.getBoundingClientRect();
    const pitch = a && b ? b.left - a.left : 404;
    drag = { id: e.pointerId, x0: e.clientX, pitch, max: Math.max(0, home.slots - home.visible), moved: false, cell: home.first, samples: [[performance.now(), e.clientX]] };
    const t = (e.target as HTMLElement).closest<HTMLElement>('.tile');
    pressedTile = t ? Number(t.dataset['index']) : null;
  }
  function onRowMove(e: PointerEvent) {
    if (!drag || e.pointerId !== drag.id) return;
    const dx = e.clientX - drag.x0;
    if (!drag.moved && Math.abs(dx) < 8) return;
    if (!drag.moved) {
      drag.moved = true;
      dragging = true;
      pressedTile = null;
      (e.currentTarget as HTMLElement).setPointerCapture?.(e.pointerId);
    }
    // 1:1, stopping at the ends.
    const pos = Math.min(Math.max(home.first * drag.pitch - dx, 0), drag.max * drag.pitch);
    dragPx = home.first * drag.pitch - pos;
    const cell = Math.round(pos / drag.pitch);
    if (cell !== drag.cell) {
      drag.cell = cell;
      onDragTick?.();
    }
    drag.samples.push([performance.now(), e.clientX]);
    if (drag.samples.length > 6) drag.samples.shift();
  }
  function onRowUp(e: PointerEvent) {
    if (!drag || e.pointerId !== drag.id) return;
    const d = drag;
    drag = null;
    pressedTile = null;
    if (!d.moved) return;
    suppressClick = true;
    setTimeout(() => (suppressClick = false), 0);
    // The release speed carries the row on, slowing over about 150 ms; then the nearest tile.
    const [t0, x0] = d.samples[0] ?? [0, 0];
    const [t1, x1] = d.samples[d.samples.length - 1] ?? [0, 0];
    const v = t1 > t0 ? (x1 - x0) / (t1 - t0) : 0; // px per ms (pointer)
    const pos = home.first * d.pitch - dragPx - v * 75; // 150 ms, slowing evenly: half of v × t
    const first = Math.min(Math.max(Math.round(pos / d.pitch), 0), d.max);
    gliding = true;
    dragging = false;
    dragPx = 0;
    onDragEnd?.(first);
    setTimeout(() => (gliding = false), 160);
  }

  // The title's natural width, measured with the canvas text API (no layout read at all, so a
  // move never forces a layout: L-16, D-007); recomputed when the title, size or scale changes.
  const measureCtx = typeof document === 'undefined' ? null : document.createElement('canvas').getContext('2d');
  $effect(() => {
    const name = shownTitle === null ? '' : (games[shownTitle]?.name ?? '');
    const font = 3.17 * 16 * px * scale; // --text-title: 3.17 rem, rem = 16 × the UI unit
    if (!measureCtx || !name) return;
    measureCtx.font = `400 ${font}px "M PLUS Rounded 1c"`;
    // + 0.04 em: the rendered text can be a pixel or two wider than the canvas measurement at
    // large sizes (alpha.11: 2 px at 2560 px, 150 %), so the box always holds the whole title.
    titleW = measureCtx.measureText(name).width + 0.04 * font;
    if (!placed) requestAnimationFrame(() => (placed = true));
  });

  // B-14: a long title holds 1.0 s at the window's left, scrolls left at 90 px/s (scaled), and
  // repeats after a gap of 4 cap heights; by transform only (L-16), one animation at a time.
  let loop: Animation | null = null;
  $effect(() => {
    loop?.cancel();
    loop = null;
    const el = titleTrack;
    if (!el || !longTitle || shownTitle === null) return;
    const font = untrack(() => 3.17 * 16 * px * scale);
    const gap = 4 * 0.73 * font;
    const speed = (90 * font) / 50.7; // 90 px/s at the measured 50.7 px title size
    const dist = titleW + gap;
    const hold = 1000;
    const dur = hold + (dist / speed) * 1000;
    loop = el.animate(
      [
        { transform: 'translateX(0)' },
        { transform: 'translateX(0)', offset: hold / dur },
        { transform: `translateX(${-dist}px)` },
      ],
      { duration: dur, iterations: Infinity, easing: 'linear' },
    );
    loop.id = 'marquee';
  });

  // B-14 (rev. q): the edge bump. Only the focus ring springs toward the pressed direction and
  // back (the tile or button stays still); transform only (L-16).
  // Runs only when a new bump arrives (its counter): it must never replay on later moves.
  $effect(() => {
    const b = bump;
    if (!b) return;
    untrack(() => playBump(b));
  });
  function playBump(b: { dir: 'left' | 'right' | 'up' | 'down'; n: number; zone: 'row' | 'drawer' }) {
    if (suspended) return;
    const horizontal = b.dir === 'left' || b.dir === 'right';
    const sign = b.dir === 'left' || b.dir === 'up' ? -1 : 1;
    const ring = document.querySelector<HTMLElement>(b.zone === 'drawer' ? '.dbtn.focused .dring' : '.tile.focused .ring');
    if (!ring) return;
    const ui = b.zone === 'drawer' ? px * (1 + (scale - 1) / 2) : px;
    const t = (v: number) => (horizontal ? `translateX(${sign * v * ui}px)` : `translateY(${sign * v * ui}px)`);
    // B-14 (rev. r, re-measured): snappy. The full push within one frame (16 ms), back past the
    // centre by about 66 ms (overshoot 6 px sideways, 4 px up / down), a 2–4 px wobble settled by
    // about 200 ms (sideways) or 180 ms (up / down).
    // The row's ends (rev. u, measured): about 14 px, overshoot about 6, settled in about 150 ms.
    const [out, back, wob, dur] = b.zone === 'row' && horizontal ? [14, 6, 2, 150] : horizontal ? [15, 6, 3, 200] : [12, 4, 2, 180];
    ring.dataset['bump'] = String(b.n);
    ring.animate(
      [
        { transform: t(0) },
        { transform: t(out), offset: 16 / dur },
        { transform: t(-back), offset: 66 / dur, easing: 'ease-in-out' },
        { transform: t(wob), offset: Math.min(0.9, 110 / dur), easing: 'ease-in-out' },
        { transform: t(-wob / 2), offset: Math.min(0.95, 135 / dur), easing: 'ease-in-out' },
        { transform: t(0) },
      ],
      { duration: dur, easing: 'linear' },
    );
  }
</script>

<svelte:window bind:innerWidth={vw} bind:innerHeight={vh} />

<section class="home" data-extras={extras.join(' ')} data-testid="home" data-owner="home" data-suspended={suspended} data-mode={mode}>
  <StatusHeader {padConnected} {status} {fixedTime} {...hour12 === undefined ? {} : { hour12 }} />

  <div class="row" data-testid="game-row" data-first={home.first} data-focus={home.focus} data-count={games.length}>
    {#if loadState === 'loaded' && shownTitle !== null && games[shownTitle]}
      {@const tg = games[shownTitle]}
      <!-- B-8 (rev. p): just like Switch, the title alone in the accent text colour, centred above
           the selected tile; moved by transform only; shifted to stay on screen near the edges; a
           long title scrolls in a window centred on the tile (B-14), clipped at its edges. -->
      <div
        class="title"
        class:placed
        style:transform={`translateX(${titleX}px)`}
        style:width={`${boxW}px`}
        data-testid="selected-title-row"
        data-index={shownTitle}
        data-shifted={shifted}
        data-owner="home-title"
      >
        <span class="title-text" data-testid="selected-title"
          >{#key tg.id}<span class="ttrack" bind:this={titleTrack} data-testid="selected-title-text" data-moving={longTitle}
              ><span class="tt">{tg.name}</span>{#if longTitle}<span class="tgap"></span><span class="tt" aria-hidden="true">{tg.name}</span>{/if}</span
            >{/key}</span
        >
      </div>
    {/if}
    {#if loadState === 'loaded'}
      <div
        class="track"
        class:wrapping
        class:dragging
        class:gliding
        style:--first={home.first}
        style:--drag={`${dragPx}px`}
        data-owner="home-row"
        data-testid="track"
        onpointerdown={onRowDown}
        onpointermove={onRowMove}
        onpointerup={onRowUp}
        onpointercancel={onRowUp}
        onlostpointercapture={onRowUp}
        role="presentation"
      >
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
            data-pointer={`tile:${i}`}
            class:focused={!suspended && mode === 'pad' && home.zone === 'row' && home.focus === i}
            class:hovered={!suspended && mode === 'mouse' && (home.hover === i || home.armed === i)}
            class:pressed={pressed === i}
            class:touched={pressedTile === i}
            onmouseenter={() => onHover(i)}
            onmouseleave={() => onHover(null)}
            onclick={() => !suppressClick && onClick(i)}
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
              {#if g.id === runningId}
                <!-- B-18: the running game's "Playing" badge (Switch adds the user's icon; AIONE has none). -->
                <span class="playing" data-testid="playing-badge">Playing</span>
              {:else if g.id === startingId}
                <!-- D-051: a start past its 45 s, still watched: "Starting…", not "Playing". -->
                <span class="playing" data-testid="starting-badge">Starting…</span>
              {/if}
            </div>
            <span class="hover-line" aria-hidden="true"></span>
            <span class="ring" aria-hidden="true" data-owner="home-ring"
              ><span class="ring-halo"></span><span class="ring-line"></span><span class="ring-line ring-alt"></span></span
            >

          </div>
        {/each}
        {#each Array.from({ length: empties }, (_, k) => games.length + k) as i (i)}
          <!-- B-14: an empty slot (focusable but the last; no title; A does nothing). -->
          <div
            class="tile empty"
            data-testid="empty-slot"
            data-index={i}
            data-pointer={i < home.slots ? `tile:${i}` : undefined}
            class:focused={!suspended && mode === 'pad' && home.zone === 'row' && home.focus === i}
            class:hovered={!suspended && mode === 'mouse' && home.hover === i}
            onmouseenter={() => i < home.slots && onHover(i)}
            onmouseleave={() => onHover(null)}
            role="presentation"
          >
            <span class="ring" aria-hidden="true" data-owner="home-ring"
              ><span class="ring-halo"></span><span class="ring-line"></span><span class="ring-line ring-alt"></span></span
            >
            <span class="hover-line" aria-hidden="true"></span>
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
            data-pointer={`drawer:${i}`}
            aria-label={item.label}
            tabindex="-1"
            onmouseenter={() => onDrawerHover(i)}
            onmouseleave={() => onDrawerHover(null)}
            onclick={() => onDrawerClick(i)}
          >
            <span class="icon dicon" style:--w={item.weight} style:color={`var(--drawer-${i + 1})`} aria-hidden="true">{item.icon}</span>
            <span class="dring" aria-hidden="true" data-owner="drawer-ring"
              ><span class="dring-halo"></span><span class="dring-line"></span><span class="dring-line dring-alt"></span></span
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
  <Footer {prompts} {device} branding {onPrompt} />
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
    top: 0;
    height: calc((291 + 386 + 16) * var(--px));
    overflow: clip;
  }
  .track {
    position: absolute;
    left: calc(155 * var(--px));
    top: calc(291 * var(--px));
    display: flex;
    gap: calc(18 * var(--px)); /* rev. x: about 18 px between pictures (pitch 404) */
    transform: translateX(calc(var(--first) * -404 * var(--px) + var(--drag, 0px)));
    /* B-16: horizontal drags are AIONE's (a finger doesn't pan the page). */
    touch-action: pan-y;
    /* B-14: the row slides one tile in about 170 ms with a strong ease-out. */
    transition: transform 170ms cubic-bezier(0.16, 1, 0.3, 1);
  }
  /* B-16: dragging follows the pointer 1:1 (no transition, no ring); the release glides to the
     nearest tile, slowing over about 150 ms. */
  .track.dragging {
    transition: none;
  }
  .track.dragging :global(.ring) {
    visibility: hidden;
  }
  .track.gliding {
    transition-duration: 150ms;
    transition-timing-function: cubic-bezier(0.2, 0.8, 0.2, 1);
  }
  /* B-16: a pale accent tint on a pressed tile (mouse or finger). */
  .tile.touched .art::after {
    content: '';
    position: absolute;
    inset: 0;
    background: var(--color-ring-peak);
    opacity: 0.22;
    pointer-events: none;
  }
  /* B-14 (rev. s): the wrap whips the row to its other end in about 200 ms (strong ease-out). */
  .track.wrapping {
    transition-duration: 200ms;
    will-change: transform;
  }
  .tile {
    position: relative;
    flex: none;
    width: calc(386 * var(--px));
    height: calc(386 * var(--px));
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

  /* B-14 / B-9 (rev. p): the focus ring is 7 px, directly around the art (no gap), and jumps
     to the next tile instantly (no fade, no slide). */
  /* B-9 (rev. v), from outside in: a soft halo (an 8 px blur in the ring's colour, swelling and
     fading), the ring (7 px, brand pink #FF46A2 always), a bright edge (2 px), a 4 px pure white
     band, the picture (13 px in all). D-020: the edge is the TOP inset shadow, so it sits next
     to the ring and the white band below it (alpha.11 had the two in the other order). */
  .ring {
    position: absolute;
    inset: calc(-13 * var(--px));
    border-radius: calc(14 * var(--px));
    opacity: 0;
    pointer-events: none;
  }
  .ring-line {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    border: calc(7 * var(--px)) solid var(--color-ring);
    box-shadow:
      inset 0 0 0 calc(2 * var(--px)) var(--color-ring-edge),
      inset 0 0 0 calc(6 * var(--px)) #ffffff;
  }
  /* B-9 (rev. x): only a thin glow at the peak: about 4 px of soft edge in the ring's colour,
     nothing at the base (its opacity follows the flash); no wide haze (alpha.12's 8 px blur and
     3 px spread are gone). It stays well clear of the title's descenders (9 px). */
  .ring-halo {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    box-shadow: 0 0 calc(4 * var(--px)) 0 var(--color-ring-peak);
    opacity: 0;
  }
  /* B-18 (measured on the user's Switch screenshot): a dark translucent rounded pill across the
     bottom of the picture: about 18 % of the tile's height, about 63 % of its width, centred,
     about 4.5 % from the bottom; "Playing" in white. */
  .playing {
    position: absolute;
    left: 50%;
    bottom: 4.5%;
    translate: -50% 0;
    width: 63%;
    height: 18%;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 999px;
    background: rgb(0 0 0 / 0.62);
    border: calc(2 * var(--px)) solid rgb(255 255 255 / 0.85);
    color: #fff;
    font: 400 calc(30 * var(--px)) / 1 var(--font-ui);
    letter-spacing: 0.01em;
    pointer-events: none;
  }
  .tile.focused .ring {
    opacity: 1;
  }
  /* B-9 (rev. m): like Switch's selection frame, the ring itself pulses smoothly between two
     shades of the accent, with no glow. Two solid rings in the two shades; only the top one's
     opacity changes (A7: opacity only), so every moment is a blend of two colours that both meet
     3:1 (D-011 stays met). 1.4 s per cycle, eased, starting after the focus-in. */
  .ring-alt {
    border-color: var(--color-ring-alt);
    opacity: 0;
  }
  /* B-9 (rev. p): one cycle every 1.0 s, restarting at the bright end on every move (the
     focused class moves, so the animation starts again); which layer is the bright one depends
     on the preset (--ring-pulse, app.css). */
  /* B-9 (rev. s): the pulse is a flash: each cycle (1.0 s) starts at the bright peak (also on
     every move: the focused class moves, so the animation restarts), fades slowly to the base
     over 0.75 s, then rises fast (0.2 s) back to the peak. The peak may drop below 3:1 against
     the background (the user's exception, like Switch); the base meets it. */
  /* The flash (rev. v): the ring only slightly lighter at the peak (#FF69B4), never pastel; the
     halo swells from almost nothing to clearly visible. Same timing (rise 0.2 s, fade 0.75 s). */
  .ring-alt {
    border-color: var(--color-ring-peak);
    box-shadow: none;
  }
  .tile.focused .ring-alt,
  .tile.focused .ring-halo {
    animation: aione-ring-flash 1000ms linear infinite;
  }
  /* B-14: the empty slots: plain, a light tint of the background with a hairline. */
  .tile.empty {
    background: var(--color-slot);
    border-radius: calc(4 * var(--px));
    box-shadow: inset 0 0 0 calc(2 * var(--px)) var(--color-slot-line);
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
  /* B-14: the title's capitals about 37 px tall (font 3.17 rem; M PLUS Rounded 1c caps are
     0.73 em), its baseline about 21 px above the ring's top (rev. v: 278), so at 257. The box's top is
     placed from the baseline: line-height 1, the font's ascent about 0.86 em (measured). */
  .title {
    position: absolute;
    left: 0;
    /* B-14: the baseline 21 px above the ring's top (rev. v: 278, with the 12.5 px ring) at 100 %; the gap scales with
       Interface Scale (like the title), so the descenders (no longer clipped, D-017) stay clear
       of the ring at 150 %. */
    top: calc(278 * var(--px) - 21 * var(--ui) - var(--title-ascent, 0.86) * var(--text-title));
    height: calc(1.4 * var(--text-title));
    /* D-017: clip sideways only (the scroll window), never below: the descenders keep their full
       depth (the font's descent); the baseline stays where B-14 puts it. */
    overflow-x: clip;
    overflow-y: visible;
    color: var(--color-title);
    font-size: var(--text-title);
    font-weight: 400;
    line-height: 1;
    white-space: nowrap;
  }
  .ttrack {
    display: inline-flex;
    will-change: transform;
  }
  .tgap {
    display: inline-block;
    width: calc(4 * 0.73em);
  }
  .title.placed {
    transition: transform var(--motion-row) var(--ease-out);
  }
  .title-text {
    display: block;
    overflow-x: clip;
    overflow-y: visible;
  }
  /* B-15 (rev. t): the extras, approved and ON by default (`--extras=none` turns all three off
     for one run). "press": the WHOLE focused tile or drawer button dips to about 94 % and springs
     back over about 0.18 s when A, a click or a tap activates it. "labelfade": the drawer label
     fades in and rises about 4 px over about 0.15 s. "titlefade": the title fades in over 120 ms.
     Transform / opacity only (L-16). */
  .home[data-extras~='press'] .tile.pressed {
    transform: none;
    animation: press-dip 180ms linear;
  }
  @keyframes press-dip {
    0% {
      transform: scale(1);
      animation-timing-function: cubic-bezier(0.3, 0, 0.6, 1);
    }
    35% {
      transform: scale(0.94);
      animation-timing-function: cubic-bezier(0.2, 0.7, 0.4, 1);
    }
    75% {
      transform: scale(1.015);
    }
    100% {
      transform: scale(1);
    }
  }
  .home[data-extras~='press'] .dbtn.pressed {
    animation: press-dip-drawer 180ms linear;
  }
  .home[data-extras~='press'] .dbtn.pressed .dicon {
    animation: none;
  }
  @keyframes press-dip-drawer {
    0% {
      transform: scale(1.08);
      animation-timing-function: cubic-bezier(0.3, 0, 0.6, 1);
    }
    35% {
      transform: scale(1.015);
      animation-timing-function: cubic-bezier(0.2, 0.7, 0.4, 1);
    }
    75% {
      transform: scale(1.096);
    }
    100% {
      transform: scale(1.08);
    }
  }
  .home[data-extras~='labelfade'] .dlabel {
    animation: label-rise 150ms ease-out;
  }
  @keyframes label-rise {
    from {
      opacity: 0;
      translate: 0 calc(4 * var(--ui));
    }
  }
  .home[data-extras~='titlefade'] .ttrack {
    animation: title-in 120ms ease-out;
  }
  @keyframes title-in {
    from {
      opacity: 0;
    }
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
      calc((291 + 386 + 87) * var(--px)), /* the disc centre at about 818 px (rev. v, measured) */
      calc(100vh - 5.5rem - 20 * var(--px) * max(0, 3 - 2 * var(--scale)) - 108 * var(--hdr) - 24.5 * var(--hdr) - 2.93rem - 8 * var(--px))
    );
    display: flex;
    justify-content: center;
    gap: calc(27 * var(--hdr)); /* rev. z: the pitch about 135 px (108 + 27) */
  }
  .ditem {
    position: relative;
  }
  /* B-14 (rev. r): the focused button grows to about 108 % (with its ring), instantly. */
  .dbtn.focused {
    transform: scale(1.08);
  }
  .dbtn {
    position: relative;
    /* The discs and icons grow at half rate with Interface Scale (like the header), so the
       drawer and its label fit between the row and the footer at 150 %; the label grows fully. */
    width: calc(108 * var(--hdr));
    height: calc(108 * var(--hdr));
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
    box-shadow: var(--disc-shadow); /* rev. z: a soft drop shadow, like Switch's */
  }
  .dicon {
    font-size: calc(58 * var(--hdr)); /* rev. v: filling more of the disc */
    line-height: 1;
    font-variation-settings:
      'wght' calc(var(--w, 300) + 200),
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
    /* rev. z (measured at rest): an 8 px ring DIRECTLY on the white button (117 px when focused,
       so 7.4 px before the 108 % scale); outer size about 133 px; no band, no edge. */
    inset: calc(-7.4 * var(--hdr));
    border-radius: 50%;
    opacity: 0;
    pointer-events: none;
  }
  .dring-line {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    border: calc(7.4 * var(--hdr)) solid var(--color-ring);
  }
  .dring-alt {
    border-color: var(--color-ring-alt);
    opacity: 0;
  }
  .dbtn.focused .dring {
    opacity: 1;
  }
  .dring-alt {
    border-color: var(--color-ring-peak);
  }
  /* B-9 (rev. w / x): the drawer's focused button has the row's ring: the white band, the bright
     edge, the thin glow and the same flash. */
  .dring-line {
    box-shadow: none;
  }
  .dring-halo {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    box-shadow: 0 0 calc(4 * var(--hdr)) 0 var(--color-ring-peak);
    opacity: 0;
  }
  .dbtn.focused .dring-halo {
    animation: aione-ring-flash 1000ms linear infinite;
  }
  .dbtn.focused .dring-alt {
    animation: aione-ring-flash 1000ms linear infinite;
  }
  .dbtn.hovered {
    box-shadow: 0 0 0 calc(4 * var(--ui)) var(--color-hover);
  }
  /* The label under the focused button, in the accent text colour, centred, never clipped. */
  .dlabel {
    position: absolute;
    /* rev. z: about 12 px below the ring (the ring's outer edge is 12.5 px below the disc's
       108 px box); the capitals start about 0.2 em into the line box. */
    top: calc(100% + 24.5 * var(--hdr) - 0.2em);
    left: 50%;
    transform: translateX(-50%);
    white-space: nowrap;
    color: var(--color-title);
    /* rev. z: the label's capitals about 31 px of ink (2.55 rem, measured), regular. */
    font-size: 2.55rem;
    font-weight: 400;
    line-height: 1.15;
    pointer-events: none;
  }

  .message {
    position: absolute;
    left: calc(155 * var(--px));
    top: calc(291 * var(--px));
    height: calc(386 * var(--px));
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
