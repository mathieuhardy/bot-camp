<script lang="ts">
  import { Badge } from '$lib/components/ui/badge';
  import type { Decision } from '$lib/types';

  let { decision }: { decision: Decision } = $props();

  const classesByDecision: Record<Decision, string> = {
    allowed: 'bg-status-allowed/15 text-status-allowed border-status-allowed/40',
    allow_listed: 'bg-status-allowed/15 text-status-allowed border-status-allowed/40',
    limited: 'bg-status-limited/15 text-status-limited border-status-limited/40',
    banned: 'bg-status-banned/15 text-status-banned border-status-banned/40',
    blocked: 'bg-status-banned/15 text-status-banned border-status-banned/40',
    trapped: 'bg-status-banned/15 text-status-banned border-status-banned/40',
  };
</script>

<Badge variant="outline" class={classesByDecision[decision]}>{decision}</Badge>
