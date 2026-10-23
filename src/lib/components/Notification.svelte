<script lang="ts">
  // T2.3: AIONE's one notification design (a game ended, a controller connected / disconnected,
  // a battery low). Passive, like Switch's: never focusable, never takes input (so B-29's edge
  // bump doesn't apply); it sits in the Home header's empty top-left spot and never covers a
  // dialog (the app holds notices while one is open). Motion: transform and opacity only (A7).
  // A16 (alpha.37): only the user's assets and existing components' values: M PLUS Rounded 1c
  // (--font-ui) at the footer's sizes; Material Symbols Rounded icons (the header's controller
  // and battery symbols, the names in notices.ts); Power Options' panel radius and shadow; palette
  // colours (B-9). No sound yet (none exists in the FilmCow family for it; asked in the handoff).
  import { fade } from 'svelte/transition';
  import type { Notice } from '../notify/notices';

  interface Props {
    notice: Notice | null;
  }
  let { notice }: Props = $props();
</script>

{#if notice}
  {#key notice.at + notice.key}
    <div class="notice" out:fade={{ duration: 200 }} role="status" aria-live="polite" data-testid="notification" data-icon={notice.icon}>
      <span class="icon badge" class:dim={notice.dim} aria-hidden="true">{notice.icon}</span>
      <span class="text">
        <span class="title" data-testid="notification-title">{notice.title}</span>
        {#if notice.detail}<span class="detail" data-testid="notification-detail">{notice.detail}</span>{/if}
      </span>
    </div>
  {/key}
{/if}

<style>
  .notice {
    position: fixed;
    /* Centred on the header's row (its items centre at 105 px, StatusHeader); the card is 92 tall. */
    top: calc(105 * var(--hdr) - 46 * var(--ui));
    left: calc(60 * var(--px));
    z-index: 60;
    display: flex;
    align-items: center;
    gap: calc(18 * var(--ui));
    max-width: calc(760 * var(--ui));
    padding: calc(16 * var(--ui)) calc(28 * var(--ui)) calc(16 * var(--ui)) calc(18 * var(--ui));
    /* Power Options' panel (PowerOptions.svelte): radius 12, shadow 0 8 28 at 25 % black. */
    border-radius: calc(12 * var(--ui));
    background: var(--color-surface);
    color: var(--color-text);
    box-shadow: 0 calc(8 * var(--ui)) calc(28 * var(--ui)) rgb(0 0 0 / 0.25);
    pointer-events: none;
    animation: notice-in var(--motion-medium) var(--ease-out) both;
  }
  .badge {
    flex: none;
    width: calc(60 * var(--ui));
    height: calc(60 * var(--ui));
    border-radius: 50%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: calc(44 * var(--ui));
    background: var(--color-soft);
    color: var(--color-hover);
  }
  .badge.dim {
    color: var(--color-icon-dim);
  }
  .text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .title {
    font: 400 var(--text-label) / 1.2 var(--font-ui);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .detail {
    font: 400 var(--text-md) / 1.2 var(--font-ui);
    color: var(--color-text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  @keyframes notice-in {
    from {
      opacity: 0;
      transform: translateY(calc(-16 * var(--ui)));
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
</style>
