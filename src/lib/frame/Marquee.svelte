<script lang="ts">
  // The shared marquee (reference A7, L-16): a width-limited box; only the inner text moves,
  // by transform (Web Animations, composited). Overflow is measured once after the text or the
  // box changes (never inside an animation or layout loop). Pause 1.2 s → scroll at 40 px/s →
  // pause → return. Short text stays still. The animation ends when the component goes away
  // (focus moves on).
  let { text, align = 'start', testid = 'marquee' }: { text: string; align?: 'start' | 'center' | 'end'; testid?: string } =
    $props();

  let box: HTMLSpanElement | undefined = $state();
  let inner: HTMLSpanElement | undefined = $state();
  let moving = $state(false);
  const PAUSE_MS = 1200;
  const SPEED = 40; // design px per second at 1080 p

  $effect(() => {
    void text;
    const b = box;
    const t = inner;
    if (!b || !t) return;
    let anim: Animation | null = null;
    const frame = requestAnimationFrame(() => {
      const overflow = t.scrollWidth - b.clientWidth;
      moving = overflow > 1;
      if (!moving) return;
      const px = window.innerHeight / 1080;
      const travel = (overflow / (SPEED * px)) * 1000;
      const total = PAUSE_MS * 2 + travel * 2;
      const at = (ms: number) => ms / total;
      anim = t.animate(
        [
          { transform: 'translateX(0)', offset: 0 },
          { transform: 'translateX(0)', offset: at(PAUSE_MS), easing: 'ease-in-out' },
          { transform: `translateX(${-overflow}px)`, offset: at(PAUSE_MS + travel) },
          { transform: `translateX(${-overflow}px)`, offset: at(PAUSE_MS * 2 + travel), easing: 'ease-in-out' },
          { transform: 'translateX(0)', offset: 1 },
        ],
        { duration: total, iterations: Infinity, id: 'marquee' },
      );
    });
    return () => {
      cancelAnimationFrame(frame);
      anim?.cancel();
    };
  });
</script>

<span class="box" bind:this={box} data-testid={testid} data-moving={moving} style:text-align={moving ? 'left' : align}
  ><span class="inner" bind:this={inner}>{text}</span></span
>

<style>
  .box {
    display: block;
    overflow: hidden;
    white-space: nowrap;
  }
  .inner {
    display: inline-block;
    white-space: nowrap;
  }
</style>
