<script lang="ts">
  import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '$lib/components/ui/card';

  // Purely presentational — mirrors docs/tutorial/api.md, kept in sync by
  // hand since bot-camp has no dynamic scenario registry.
  const endpoints: { path: string; description: string }[] = [
    { path: 'GET /status/{code}', description: 'Return any HTTP status code from 100 to 999.' },
    { path: 'GET /headers/echo', description: 'Echo every header received.' },
    { path: 'GET /headers/set', description: 'Force arbitrary response headers.' },
    { path: 'GET /delay/{ms}', description: 'Delay the response by a fixed duration.' },
    { path: 'GET /large-response/{kb}', description: 'Return a body of a controlled size.' },
    { path: 'GET /auth/basic', description: 'HTTP Basic Auth challenge.' },
    { path: 'GET /redirect/{code}', description: 'Redirect with a chosen status code.' },
    { path: 'GET /redirect/chain', description: 'Chain of redirects, n hops deep.' },
    { path: 'GET /redirect/loop', description: 'Redirect loop back to the start.' },
    { path: 'GET /redirect/refresh', description: 'Refresh response header instead of a real redirect.' },
    { path: 'GET /redirect/meta-refresh', description: 'Meta-refresh tag instead of a real redirect.' },
    { path: 'GET /canonical', description: 'Canonical link tag, with conflicting variants.' },
    { path: 'GET /normalize', description: 'Redirect a URL to its normalized form.' },
    { path: 'GET /robots.txt', description: 'Dynamic robots.txt, configurable via PUT.' },
    { path: 'GET /robots/meta', description: 'Meta robots and X-Robots-Tag, including conflicts.' },
    { path: 'GET /content', description: 'Title/H1/word-count/duplicate-content scenarios.' },
    { path: 'GET /js-render', description: 'Signals injected into the DOM after a delay.' },
    { path: 'GET /encoding', description: 'Charset mismatches and double-encoding.' },
    { path: 'GET /broken-html', description: 'Deliberately malformed HTML markup.' },
    { path: 'GET /ratelimit/{*path}', description: 'Playground gated by the rate limiter.' },
    { path: 'GET /honeypot/{*path}', description: 'Trap that bans on first visit.' },
    { path: 'GET /challenge/{*path}', description: '"Checking your browser" JS challenge gate.' },
  ];
</script>

<Card>
  <CardHeader>
    <CardTitle>Available endpoints</CardTitle>
    <CardDescription>See docs/tutorial/api.md for the full reference, including admin endpoints.</CardDescription>
  </CardHeader>

  <CardContent>
    <ul class="grid grid-cols-1 gap-1.5 text-sm sm:grid-cols-2">
      {#each endpoints as endpoint (endpoint.path)}
        <li class="flex flex-col">
          <span class="font-mono text-xs text-foreground">{endpoint.path}</span>
          <span class="text-xs text-muted-foreground">{endpoint.description}</span>
        </li>
      {/each}
    </ul>
  </CardContent>
</Card>
