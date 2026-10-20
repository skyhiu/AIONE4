<script lang="ts">
  // B-26 rev. ay: AIONE's one slider design, like Switch's (measured from the user's photo at
  // 1080p / 100 %): a thin track about 6 px high; a large round knob about 44 px across sitting on
  // it (a soft shadow on the light preset, a soft light outline on the dark one); the filled part in
  // the accent #FF46A2, the rest a dim grey from the preset. It moves by transform: a press glides
  // (about 120 ms, eased); a held direction is driven frame by frame (`glide` off), so never jumps.
  let { value, glide = true, testid, label }: { value: number; glide?: boolean; testid: string; label: string } = $props();
  const v = $derived(Math.max(0, Math.min(100, value)));
</script>

<div class="slider" class:glide data-testid={testid} data-value={Math.round(v)} role="slider" aria-label={label} aria-valuenow={Math.round(v)} aria-valuemin="0" aria-valuemax="100" style:--v={v}>
  <div class="track"></div>
  <div class="fill"></div>
  <div class="knob-run"><div class="knob"></div></div>
</div>

<style>
  .slider {
    position: relative;
    flex: 1;
    height: calc(44 * var(--px, 1px));
    margin: 0 calc(22 * var(--px, 1px));
  }
  .track,
  .fill {
    position: absolute;
    left: 0;
    right: 0;
    top: 50%;
    height: calc(6 * var(--px, 1px));
    translate: 0 -50%;
    border-radius: calc(3 * var(--px, 1px));
  }
  .track {
    background: var(--color-slider-rest, rgb(128 128 128 / 0.4));
  }
  .fill {
    background: #ff46a2;
    transform-origin: left center;
    transform: scaleX(calc(var(--v) / 100));
  }
  /* The knob rides a full-width runner: translating the runner by v % of its own width puts the
     knob's centre at v % of the track. */
  .knob-run {
    position: absolute;
    inset: 0;
    transform: translateX(calc(var(--v) * 1%));
  }
  .knob {
    position: absolute;
    left: calc(-22 * var(--px, 1px));
    top: 0;
    width: calc(44 * var(--px, 1px));
    height: calc(44 * var(--px, 1px));
    border-radius: 50%;
    background: var(--color-surface, #fff);
    box-shadow: 0 calc(2 * var(--px, 1px)) calc(6 * var(--px, 1px)) rgb(0 0 0 / 0.28);
  }
  :global(:root[data-preset='dark']) .knob {
    background: #e8e8e8;
    box-shadow: 0 0 0 calc(2 * var(--px, 1px)) rgb(255 255 255 / 0.35);
  }
  .glide .fill,
  .glide .knob-run {
    transition: transform 120ms cubic-bezier(0.2, 0.8, 0.2, 1);
  }
</style>
