<script lang="ts">
  // The Home status header (reference A4; real Switch Home, inspired by, not copied): the clock
  // in a regular weight with slightly wide spacing and a smaller AM / PM, then the controller,
  // network and battery icons, all one optical size and ink (L-10), light and outlined, with
  // generous space between items. Scales at half the rate of Interface Scale (--hdr).
  // Timers: one setTimeout to the next minute (owner: header clock).
  import { onMount } from 'svelte';
  import { formatClock, msToNextMinute, systemUses12h, type ClockText } from '../clock';
  import type { PcStatus } from '../core/types';

  let {
    padConnected,
    status,
    hour12 = systemUses12h(),
    fixedTime = null,
  }: {
    padConnected: boolean;
    status: PcStatus | null;
    hour12?: boolean;
    /** Tests: a fixed time instead of the real clock. */
    fixedTime?: Date | null;
  } = $props();

  let now = $state(new Date());
  const clock: ClockText = $derived(formatClock(fixedTime ?? now, hour12));

  onMount(() => {
    if (fixedTime) return;
    let timer: ReturnType<typeof setTimeout>;
    const tick = () => {
      now = new Date();
      timer = setTimeout(tick, msToNextMinute(now) + 20);
    };
    timer = setTimeout(tick, msToNextMinute(now) + 20);
    return () => clearTimeout(timer);
  });

  function batteryIcon(b: NonNullable<PcStatus['battery']>): string {
    if (b.charging) return 'battery_android_frame_bolt';
    if (b.percent === null) return 'battery_android_frame_question';
    if (b.percent >= 95) return 'battery_android_frame_full';
    return `battery_android_frame_${Math.max(0, Math.min(6, Math.round((b.percent / 100) * 6)))}`;
  }

  // L-10 / KD-81: Material Symbols draw their glyphs at different visible heights. Each icon
  // gets an optical factor so its ink is as tall as the clock's capitals (measured at 1080 p:
  // Wi-Fi 18 px = the caps; controller 15 px; horizontal battery 13 px).
  const OPTICAL: Record<string, number> = { sports_esports: 1.2, battery: 1.45, lan: 0.8, wifi_off: 0.92, wifi: 0.96 };
  const optical = (name: string) => OPTICAL[name.startsWith('battery') ? 'battery' : name] ?? 1;

  const networkIcon = $derived(
    status?.network === 'wired' ? 'lan' : status?.network === 'offline' ? 'wifi_off' : status?.network === 'wifi' ? 'wifi' : null,
  );
</script>

<header class="status" data-testid="status-header" data-owner="header">
  <span class="clock" data-testid="clock"
    ><span class="digits">{clock.time}</span>{#if clock.ampm}<span class="ampm" data-testid="ampm">{clock.ampm}</span
      >{/if}</span
  >
  <span
    class="icon"
    class:dim={!padConnected}
    data-testid="status-controller"
    data-connected={padConnected}
    style:--optical={optical('sports_esports')}
    aria-label={padConnected ? 'Controller connected' : 'No controller connected'}>sports_esports</span
  >
  {#if networkIcon}
    <span
      class="icon"
      data-testid="status-network"
      data-network={status?.network}
      style:--optical={optical(networkIcon)}
      aria-label="Network: {status?.network}"
      >{networkIcon}</span
    >
  {/if}
  {#if status?.battery}
    <span
      class="icon"
      data-testid="status-battery"
      style:--optical={optical('battery')}
      aria-label="Battery {status.battery.percent ?? 'unknown'} %{status.battery.charging ? ', charging' : ''}"
      >{batteryIcon(status.battery)}</span
    >
  {/if}
</header>

<style>
  .status {
    position: absolute;
    /* Cluster centre about 8.5 % of the height from the top, right edge about 5 % of the
       width in (reference A4 measurements), moving with the same damped factor. */
    top: calc(91.8 * var(--hdr) - var(--header-icon) / 2);
    right: 5vw;
    display: flex;
    align-items: center;
    gap: calc(26 * var(--hdr));
    color: var(--color-text);
    height: var(--header-icon);
  }
  .clock {
    font-size: var(--header-text);
    font-weight: 400;
    letter-spacing: 0.06em;
    line-height: 1;
    font-variant-numeric: tabular-nums;
    display: inline-flex;
    align-items: baseline;
  }
  .ampm {
    font-size: 0.6em;
    letter-spacing: 0.04em;
    margin-left: 0.12em;
  }
  .icon {
    font-size: calc(var(--header-icon) * var(--optical, 1));
    min-width: var(--header-icon);
    height: var(--header-icon);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    transition: opacity var(--motion-medium) var(--ease-in-out);
  }
  .dim {
    color: var(--color-icon-dim);
  }
</style>
