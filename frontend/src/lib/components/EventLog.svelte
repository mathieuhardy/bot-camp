<script lang="ts">
  import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '$lib/components/ui/card';
  import { ScrollArea } from '$lib/components/ui/scroll-area';
  import DecisionBadge from './DecisionBadge.svelte';
  import { dashboard } from '$lib/state.svelte';

  function formatTime(timestampMs: number): string {
    return new Date(timestampMs).toLocaleTimeString();
  }
</script>

<Card>
  <CardHeader>
    <CardTitle>Live events</CardTitle>
    <CardDescription>Last {dashboard.events.length} decision{dashboard.events.length === 1 ? '' : 's'}.</CardDescription>
  </CardHeader>

  <CardContent>
    <ScrollArea class="h-80">
      {#if dashboard.events.length === 0}
        <p class="text-sm text-muted-foreground">Nothing happened yet — hit /ratelimit/* or /honeypot/* to see events stream in.</p>
      {:else}
        <ul class="flex flex-col gap-1.5 text-sm">
          {#each dashboard.events as event (event.timestamp_ms + event.source + event.key + event.decision)}
            <li class="flex items-center gap-2">
              <span class="w-20 shrink-0 font-mono text-xs text-muted-foreground">{formatTime(event.timestamp_ms)}</span>
              <span class="w-20 shrink-0 text-xs text-muted-foreground">{event.source}</span>
              <DecisionBadge decision={event.decision} />
              <span class="truncate font-mono text-xs">{event.key}</span>
            </li>
          {/each}
        </ul>
      {/if}
    </ScrollArea>
  </CardContent>
</Card>
