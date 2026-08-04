<script lang="ts">
  import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '$lib/components/ui/card';
  import { dashboard } from '$lib/state.svelte';

  const config = $derived(dashboard.challengeConfig);
</script>

<Card>
  <CardHeader>
    <CardTitle>JS challenge</CardTitle>
    <CardDescription>No per-key state — only the current configuration.</CardDescription>
  </CardHeader>

  <CardContent>
    {#if config}
      <dl class="grid grid-cols-2 gap-2 text-sm">
        <dt class="text-muted-foreground">Delay before solving</dt>
        <dd>{config.delay_ms}ms</dd>
        <dt class="text-muted-foreground">Cookie max age</dt>
        <dd>{config.cookie_max_age_secs}s</dd>
      </dl>
    {:else}
      <p class="text-sm text-muted-foreground">Waiting for the initial snapshot&hellip;</p>
    {/if}
  </CardContent>
</Card>
