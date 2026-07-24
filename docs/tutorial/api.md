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
