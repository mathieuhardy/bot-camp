<script lang="ts">
  import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '$lib/components/ui/card';
  import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '$lib/components/ui/table';
  import DecisionBadge from './DecisionBadge.svelte';
  import { dashboard } from '$lib/state.svelte';

  const config = $derived(dashboard.honeypotConfig);
  const keys = $derived([...dashboard.honeypotKeys.values()].sort((a, b) => a.key.localeCompare(b.key)));
</script>

<Card>
  <CardHeader>
    <CardTitle>Honeypot</CardTitle>
    <CardDescription>
      {#if config}
        keyed by {config.key_strategy} &middot; ban for {Math.round(config.ban_duration_ms / 1000)}s
      {:else}
        Waiting for the initial snapshot&hellip;
      {/if}
    </CardDescription>
  </CardHeader>

  <CardContent>
    {#if keys.length === 0}
      <p class="text-sm text-muted-foreground">No key trapped yet.</p>
    {:else}
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Key</TableHead>
            <TableHead>Status</TableHead>
            <TableHead>Retry after</TableHead>
          </TableRow>
        </TableHeader>

        <TableBody>
          {#each keys as entry (entry.key)}
            <TableRow>
              <TableCell class="font-mono text-xs">{entry.key}</TableCell>
              <TableCell><DecisionBadge decision="trapped" /></TableCell>
              <TableCell>{entry.retry_after_secs !== null ? `${entry.retry_after_secs}s` : '—'}</TableCell>
            </TableRow>
          {/each}
        </TableBody>
      </Table>
    {/if}
  </CardContent>
</Card>
