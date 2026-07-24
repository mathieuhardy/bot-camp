# API reference

Every route exposed by bot-camp, documented in full. This file is updated
as each roadmap phase (see the [README](../../README.md)) adds new
routes.

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
