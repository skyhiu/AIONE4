<script lang="ts">
  // T2.3: AIONE's one notification design (a game ended, a controller connected / disconnected,
  // a battery low). Passive, like Switch's: never focusable, never takes input (so B-29's edge
  // bump doesn't apply); it sits in the Home header's empty top-left spot and never covers a
  // dialog (the app holds notices while one is open). Motion: transform and opacity only (A7).
  import { fade } from 'svelte/transition';
  import type { Notice } from '../notify/notices';

  interface Props {
    notice: Notice | null;
  }
  let { notice }: Props = $props();

  const ICONS: Record<Notice['icon'], string> = {
    game: 'sports_esports',
    pad: 'videogame_asset',
    padOff: 'videogame_asset_off',
    battery: 'battery_alert',
  };
</script>

{#if notice}
  {#key notice.at + notice.key}
    <div class="notice" out:fade={{ duration: 200 }} role="status" aria-live="polite" data-testid="notification" data-kind={notice.icon}>
      <span class="icon badge" aria-hidden="true">{ICONS[notice.icon]}</span>
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
    border-radius: calc(20 * var(--ui));
    background: var(--color-surface);
    color: var(--color-text);
    box-shadow: 0 calc(6 * var(--ui)) calc(22 * var(--ui)) rgba(0, 0, 0, 0.18);
    border: calc(1 * var(--ui)) solid transparent;
    pointer-events: none;
    animation: notice-in var(--motion-medium, 220ms) var(--ease-out, cubic-bezier(0.2, 0.8, 0.2, 1)) both;
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
  .text {
    display: flex;
    flex-direction: column;
    min-width: 0;
    line-height: 1.2;
  }
  .title {
    font-size: var(--text-label);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .detail {
    font-size: var(--text-md);
    color: var(--color-text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* The dark preset: the shadow doesn't show on #2D2D2D; a quiet outline separates the card. */
  :global(:root[data-preset='dark']) .notice {
    border-color: var(--color-divider);
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
