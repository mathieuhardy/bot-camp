<script lang="ts">
  import { onMount } from 'svelte';
  import { connect } from '$lib/socket';
  import { dashboard } from '$lib/state.svelte';
  import EndpointList from '$lib/components/EndpointList.svelte';
  import RateLimitPanel from '$lib/components/RateLimitPanel.svelte';
  import HoneypotPanel from '$lib/components/HoneypotPanel.svelte';
  import ChallengePanel from '$lib/components/ChallengePanel.svelte';
  import EventLog from '$lib/components/EventLog.svelte';
  import { Separator } from '$lib/components/ui/separator';

  onMount(() => {
    connect();
  });
</script>

<div class="mx-auto flex max-w-6xl flex-col gap-6 p-6">
  <header class="flex items-center justify-between">
    <div>
      <h1 class="text-xl font-semibold">bot-camp dashboard</h1>
      <p class="text-sm text-muted-foreground">Live view of the rate limiter, honeypot and JS challenge.</p>
    </div>

    <div class="flex items-center gap-2 text-sm">
      <span
        class="size-2 rounded-full {dashboard.connected ? 'bg-status-allowed' : 'bg-status-banned'}"
        aria-hidden="true"
      ></span>
      {dashboard.connected ? 'Connected' : 'Reconnecting…'}
    </div>
  </header>

  <Separator />

  <EndpointList />

  <div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
    <RateLimitPanel />
    <HoneypotPanel />
    <ChallengePanel />
    <EventLog />
  </div>
</div>
