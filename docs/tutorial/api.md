# API reference

Every route exposed by bot-camp, documented in full. This file is updated
as each roadmap phase (see the [README](../../README.md)) adds new
routes.

## `GET /auth/basic`

HTTP Basic Auth challenge. Checks that a crawler handles a `401` cleanly
(no crash, no infinite retry, no indexing of the challenge page), and
lets you configure your own crawler with the credentials below to verify
it can also authenticate successfully when told to.

The valid credentials are published here on purpose — the point is to
test known-good and known-bad paths, not to make anything guess them:
username `bot-camp`, password `bot-camp`.

**Request**

| Parameter | Type | Description |
|---|---|---|
| `Authorization` header | `Basic <base64(username:password)>` | Optional. Omit it to trigger the challenge. |

**Response**

| Status | Body | When |
|---|---|---|
| `200 OK` | *(empty)* | `Authorization` holds the valid `bot-camp:bot-camp` credentials. |
| `401 Unauthorized` | *(empty)*, with a `WWW-Authenticate: Basic realm="bot-camp"` header | `Authorization` is missing, malformed, or holds the wrong credentials. |

**Examples**

```sh
curl -i http://localhost:3000/auth/basic
```

```
HTTP/1.1 401 Unauthorized
www-authenticate: Basic realm="bot-camp"
content-length: 0

```

```sh
curl -i -u bot-camp:bot-camp http://localhost:3000/auth/basic
```

```
HTTP/1.1 200 OK
content-length: 0

```

## `GET /canonical`

Returns an HTML page carrying a `<link rel="canonical">` tag, to test how
a crawler handles the classic canonicalization edge cases: self-vs-cross
page, relative vs absolute, duplicated, placed outside `<head>`, or
conflicting with `og:url`.

**Request**

| Parameter | Type | Description |
|---|---|---|
| `to` | query, string | The canonical URL. Pass the page's own URL for a self-referential canonical, or another page's URL to test cross-page canonicalization; relative or absolute, whatever `to` holds. |
| `og_url` | query, string | Optional. Adds a conflicting `<meta property="og:url">` tag. |
| `duplicate` | query, bool | Optional, defaults to `false`. Emits the canonical tag twice. |
| `in_body` | query, bool | Optional, defaults to `false`. Moves the canonical tag(s) into `<body>` instead of `<head>` — an invalid placement a crawler should reject. |

**Response**

| Status | Body | When |
|---|---|---|
| `200 OK` | The rendered HTML page | Always — the handler cannot fail. |

**Examples**

```sh
curl -s "http://localhost:3000/canonical?to=/page"
```

```html
<!doctype html>
<html>
<head>
<title>Canonical</title>
<link rel="canonical" href="/page">
</head>
<body>
Canonical tag test page.
</body>
</html>
```

```sh
curl -s "http://localhost:3000/canonical?to=/page&duplicate=true&in_body=true&og_url=/other"
```

```html
<!doctype html>
<html>
<head>
<title>Canonical</title>
<meta property="og:url" content="/other">
</head>
<body>
Canonical tag test page.
<link rel="canonical" href="/page">
<link rel="canonical" href="/page">
</body>
</html>
```

## `GET /delay/{ms}`

Waits `ms` milliseconds before responding, to simulate a slow page load.

**Request**

| Parameter | Type | Description |
|---|---|---|
| `ms` | path, `u64` | The delay, in milliseconds, before the response is sent. |

**Response**

| Status | Body | When |
|---|---|---|
| `200 OK` | *(empty)*, sent after the requested delay | Always — the handler cannot fail. |

**Example**

```sh
curl -i http://localhost:3000/delay/20
```

```
HTTP/1.1 200 OK
content-length: 0

```

## `GET /headers/echo`

Returns every header received on the request, as JSON. Handy to check
exactly what a crawler sends (`User-Agent`, `Accept-Language`, custom
headers, etc.).

**Request**

No parameters, no body.

**Response**

| Status | Body | When |
|---|---|---|
| `200 OK` | JSON object: header name → array of its values | Always — the handler cannot fail. |

Header names are lower-cased. A header sent on several lines (e.g. the
same name twice) is grouped under one key, values in receive order.

**Example**

```sh
curl -H "X-Foo: bar" -H "X-Foo: baz" http://localhost:3000/headers/echo
```

```json
{"accept": ["*/*"], "host": ["localhost:3000"], "x-foo": ["bar", "baz"]}
```

## `GET /headers/set`

Sets arbitrary response headers from the query string. Useful to
simulate malformed or unusual header values (a bad `Content-Type`, a
stray `X-Robots-Tag`, `HSTS`, etc.) that a crawler must tolerate.

**Request**

| Parameter | Type | Description |
|---|---|---|
| any query param | string | Becomes a response header of the same name/value. Repeat a name to emit several header lines for it. |

**Response**

| Status | Body | When |
|---|---|---|
| `200 OK` | *(empty)*, with the requested headers attached | Every name/value from the query string is a valid HTTP header. |
| `400 Bad Request` | Error message | A name or value isn't valid for an HTTP header (e.g. contains a space or a newline). |

**Examples**

```sh
curl -i "http://localhost:3000/headers/set?x-foo=bar&x-foo=baz"
```

```
HTTP/1.1 200 OK
x-foo: bar
x-foo: baz
content-length: 0

```

```sh
curl -i "http://localhost:3000/headers/set?x-foo=bad%0Avalue"
```

```
HTTP/1.1 400 Bad Request
content-length: ...

failed to parse header value
```

## `GET /health`

Health check endpoint. Used to verify the server process is up, e.g. for
container orchestration liveness probes.

**Request**

No parameters, no body.

**Response**

| Status | Body | When |
|---|---|---|
| `200 OK` | *(empty)* | Always — the handler cannot fail. |

**Example**

```sh
curl -i http://localhost:3000/health
```

```
HTTP/1.1 200 OK
content-length: 0

```

## `GET /large-response/{kb}`

Returns a response body of a controlled size, to test how a crawler
handles very large or very small pages.

**Request**

| Parameter | Type | Description |
|---|---|---|
| `kb` | path, `u64` | The size of the response body, in kilobytes. |

**Response**

| Status | Body | When |
|---|---|---|
| `200 OK` | `kb * 1024` bytes, filled with `'a'` characters | Always — the handler cannot fail. |

**Example**

```sh
curl -i http://localhost:3000/large-response/1
```

```
HTTP/1.1 200 OK
content-type: text/plain; charset=utf-8
content-length: 1024

aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa...
```

## `GET /normalize`

Normalizes a URL and redirects to the result — the way a real server
canonicalizes a URL (lowercasing the host, dropping a trailing slash,
etc.) via a plain `3xx` — so a crawler exercises its normal
redirect-following path instead of having to parse a bespoke response
format. Useful to check whether a crawler's own normalization agrees
with this reference implementation.

Scheme, host case, default port (`:80`/`:443`), and path dot-segments
(`.`, `..`) are always normalized — that's inherent to URL parsing and
can't be turned off. The remaining rules are each togglable
independently, so you can inspect what a single rule changes in
isolation. Path segments and query keys/values keep their case unchanged
either way, since case sensitivity there is meaningful.

**Request**

| Parameter | Type | Description |
|---|---|---|
| `url` | query, string | The URL to normalize. Must be absolute (a scheme and host are required). |
| `remove_host_dots` | query, bool | Optional, defaults to `true`. Collapses leading, trailing, and duplicated dots in the host (`example.com..` -> `example.com`). |
| `remove_trailing_slash` | query, bool | Optional, defaults to `true`. Strips a single trailing slash from the path, unless the path is just `/`. |
| `sort_query` | query, bool | Optional, defaults to `true`. Sorts query parameters alphabetically. |
| `remove_fragment` | query, bool | Optional, defaults to `true`. Drops the fragment (`#...`) entirely. |

**Response**

| Status | Body | When |
|---|---|---|
| `301 Moved Permanently` | *(empty)*, with a `Location` header pointing at the normalized URL | The normalized URL differs from `url`. |
| `200 OK` | *(empty)* | `url` was already fully normalized — nothing to redirect to. |
| `400 Bad Request` | Error message | `url` doesn't parse as a URL. |

**Examples**

```sh
curl -i "http://localhost:3000/normalize?url=HTTP://ExAmPle.COM:80/a/./b/../c/?d=2&c=1#frag"
```

```
HTTP/1.1 301 Moved Permanently
location: http://example.com/a/c?c=1&d=2
content-length: 0

```

```sh
curl -i "http://localhost:3000/normalize?url=http://example.com/path?c=3&a=1&b=2&sort_query=false"
```

```
HTTP/1.1 200 OK
content-length: 0

```

## `GET /redirect/{code}`

Redirects to an arbitrary URL with a given redirect status code. Useful
to check that a crawler follows redirects correctly and treats each
status code according to its own semantics (e.g. whether the method is
preserved).

**Request**

| Parameter | Type | Description |
|---|---|---|
| `code` | path, `u16` | The redirect status code: `300`, `301`, `302`, `303`, `307`, or `308`. |
| `to` | query, string | The URL to redirect to, absolute or relative. |

**Response**

| Status | Body | When |
|---|---|---|
| `{code}` | *(empty)*, with a `Location: {to}` header | `code` is a valid redirect status and `to` is a valid header value. |
| `400 Bad Request` | Error message | `code` isn't one of the redirect statuses above, or `to` isn't a valid header value. |

**Examples**

```sh
curl -i "http://localhost:3000/redirect/301?to=/status/200"
```

```
HTTP/1.1 301 Moved Permanently
location: /status/200
content-length: 0

```

```sh
curl -i "http://localhost:3000/redirect/200?to=/status/200"
```

```
HTTP/1.1 400 Bad Request
content-length: ...

not a redirect status code: 200
```

## `GET /redirect/chain`

Redirects through a configurable number of intermediate hops before
landing on a final URL, to test that a crawler correctly follows a
redirect chain.

**Request**

| Parameter | Type | Description |
|---|---|---|
| `n` | query, `u32` | Number of hops remaining before landing on `to`. |
| `to` | query, string | The URL to land on once the chain completes. |

**Response**

| Status | Body | When |
|---|---|---|
| `302 Found` | *(empty)*, with a `Location` header pointing either at `/redirect/chain?n={n-1}&to={to}` (while `n` is positive) or directly at `to` (once `n` reaches `0`) | `to` is a valid header value. |
| `400 Bad Request` | Error message | `to` isn't a valid header value. |

**Example**

```sh
curl -i "http://localhost:3000/redirect/chain?n=2&to=/status/200"
```

```
HTTP/1.1 302 Found
location: /redirect/chain?n=1&to=/status/200
content-length: 0

```

## `GET /redirect/loop`

Redirects forever, cycling through a configurable number of positions,
to test that a crawler detects and breaks out of a redirect loop instead
of following it endlessly. `steps=1` is an immediate self-loop; `steps=2`
is the classic A→B→A case.

**Request**

| Parameter | Type | Description |
|---|---|---|
| `steps` | query, `u32` | Total number of positions in the loop. Must be at least `1`. |
| `step` | query, `u32` | Current position in the loop. Defaults to `0`. |

**Response**

| Status | Body | When |
|---|---|---|
| `302 Found` | *(empty)*, with a `Location` header pointing at `/redirect/loop?steps={steps}&step={(step+1) % steps}` | `steps` is at least `1`. |
| `400 Bad Request` | Error message | `steps` is `0`. |

**Example**

```sh
curl -i "http://localhost:3000/redirect/loop?steps=2&step=1"
```

```
HTTP/1.1 302 Found
location: /redirect/loop?steps=2&step=0
content-length: 0

```

## `GET /redirect/meta-refresh`

Redirects to a URL via an HTML `<meta http-equiv="refresh">` tag instead
of a real `3xx` status or the `Refresh` header — the HTML-level twin of
`/redirect/refresh`. Useful to check whether a crawler parses the
`<head>` for this legacy redirect mechanism.

**Request**

| Parameter | Type | Description |
|---|---|---|
| `delay` | query, `u64` | Delay, in seconds, announced in the meta-refresh tag. |
| `to` | query, string | The URL to redirect to. |

**Response**

| Status | Body | When |
|---|---|---|
| `200 OK` | An HTML page whose `<head>` holds `<meta http-equiv="refresh" content="{delay}; url={to}">` | Always — the handler cannot fail. |

**Example**

```sh
curl -s "http://localhost:3000/redirect/meta-refresh?delay=5&to=/status/200"
```

```html
<!doctype html>
<html>
<head>
<title>Meta refresh</title>
<meta http-equiv="refresh" content="5; url=/status/200">
</head>
<body>
Redirecting…
</body>
</html>
```

## `GET /redirect/refresh`

Redirects to a URL via a `Refresh` response header instead of a real
`3xx` status, the way old-school "you will be redirected in N seconds"
pages do. Useful to check whether a crawler recognizes this
non-standard, header-based redirect mechanism in addition to real
`Location`-based redirects.

**Request**

| Parameter | Type | Description |
|---|---|---|
| `delay` | query, `u64` | Delay, in seconds, announced in the `Refresh` header. |
| `to` | query, string | The URL to redirect to. |

**Response**

| Status | Body | When |
|---|---|---|
| `200 OK` | *(empty)*, with a `Refresh: {delay}; url={to}` header | `to` is a valid header value. |
| `400 Bad Request` | Error message | `to` isn't a valid header value. |

**Example**

```sh
curl -i "http://localhost:3000/redirect/refresh?delay=5&to=/status/200"
```

```
HTTP/1.1 200 OK
refresh: 5; url=/status/200
content-length: 0

```

## `GET /robots.txt`

Returns the `robots.txt` contents currently held in memory. Unlike every
other route in bot-camp, a crawler always requests `robots.txt` at that
exact path, with no query string — so its content can't be steered
through the URL the way `/canonical` or `/normalize` are. Configure it
first with `PUT /robots.txt`, then point your crawler at the site.

**Request**

No parameters, no body.

**Response**

| Status | Body | When |
|---|---|---|
| `200 OK` | The current `robots.txt` contents (`text/plain`) | Always — the handler cannot fail. Defaults to `User-agent: *\nAllow: /\n` until a `PUT` sets it. |

**Example**

```sh
curl -s http://localhost:3000/robots.txt
```

```
User-agent: *
Allow: /
```

## `PUT /robots.txt`

Overwrites the contents served by `GET /robots.txt` with the request
body, verbatim — no parsing, no validation. Craft whichever edge case
you want to test directly in the body: an empty line in the middle, an
`Allow` longer/shorter/equal to its `Disallow`, duplicated directives,
mixed casing, several `User-agent` groups, a `Crawl-delay`, a
`Sitemap:` line.

**Request**

| Parameter | Type | Description |
|---|---|---|
| Request body | text | The exact contents to serve back from `GET /robots.txt`. |

**Response**

| Status | Body | When |
|---|---|---|
| `200 OK` | *(empty)* | Always — the handler cannot fail. |

**Example**

```sh
curl -i -X PUT http://localhost:3000/robots.txt --data-binary $'User-agent: Googlebot\nDisallow: /private\n'
```

```
HTTP/1.1 200 OK
content-length: 0

```

## `GET /robots/meta`

Returns an HTML page carrying a `<meta name="robots">` tag, and
optionally an `X-Robots-Tag` response header, to test the classic
meta-robots edge cases: directive combinations, case variations,
duplication, and a deliberate conflict between the meta tag and the
header.

**Request**

| Parameter | Type | Description |
|---|---|---|
| `directives` | query, string | The `content` attribute value, verbatim — e.g. `noindex,nofollow`, in whatever casing or combination you want to test. |
| `x_robots_tag` | query, string | Optional. Sets an `X-Robots-Tag` response header with its own value, to test a meta-tag/header conflict. |
| `duplicate` | query, bool | Optional, defaults to `false`. Emits the meta tag twice. |

**Response**

| Status | Body | When |
|---|---|---|
| `200 OK` | The rendered HTML page, with the `X-Robots-Tag` header if `x_robots_tag` was given | `x_robots_tag` is a valid header value, or absent. |
| `400 Bad Request` | Error message | `x_robots_tag` isn't a valid header value. |

**Examples**

```sh
curl -s "http://localhost:3000/robots/meta?directives=noindex,nofollow"
```

```html
<!doctype html>
<html>
<head>
<title>Robots meta</title>
<meta name="robots" content="noindex,nofollow">
</head>
<body>
Robots meta tag test page.
</body>
</html>
```

```sh
curl -i "http://localhost:3000/robots/meta?directives=index&x_robots_tag=noindex"
```

```
HTTP/1.1 200 OK
x-robots-tag: noindex
content-length: ...

```

## `GET /status/{code}`

Returns the requested HTTP status code. Useful to check how a crawler
reacts to any given code, including non-standard ones.

**Request**

| Parameter | Type | Description |
|---|---|---|
| `code` | path, `u16` | The HTTP status code to respond with. |

**Response**

| Status | Body | When |
|---|---|---|
| `{code}` | *(empty)* | `code` is a valid HTTP status code (100-999). |
| `400 Bad Request` | Error message | `code` isn't in the 100-999 range, or isn't a number. |

**Examples**

```sh
curl -i http://localhost:3000/status/404
```

```
HTTP/1.1 404 Not Found
content-length: 0

```

```sh
curl -i http://localhost:3000/status/999
```

```
HTTP/1.1 999
content-length: 0

```

```sh
curl -i http://localhost:3000/status/1000
```

```
HTTP/1.1 400 Bad Request
content-length: ...

invalid status code
```
