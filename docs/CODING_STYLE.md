# Coding style guide

This document describes a set of Rust style conventions. The goal is that
a new file or a new contribution stays indistinguishable from the rest of
the existing code.

Where this guide conflicts with `rustfmt`/`clippy`, the automated tools
always win on mechanical formatting (indentation, line width, etc.). This
guide covers what those tools don't decide: intentional blank lines, doc
comments, file organization.

## 1. Blank lines

**Rule: the presence of a "decoration" (a `///` doc comment, or an
attribute such as `#[cfg(...)]`, `#[error(...)]`) above an item triggers a
blank line after that item. A bare item, with no decoration, stays packed
against its neighbors.**

This explains the reasoning behind the rule: it's not "add whitespace for
whitespace's sake", it's "give breathing room to what's documented/
annotated" so that each block (doc + item) reads as a visually separate
unit from the others.

### Struct fields

No doc → no blank line, fields packed tightly:

```rust
struct Client {
    id: Uuid,
    endpoint: String,
    timeout_ms: u64,
    // ...
}
```

A `///` on each field → a blank line after *every* field, including the
last one before `}` if a comment follows in another block:

```rust
pub struct ServerConfig {
    /// Port the server binds to.
    pub port: u16,

    /// Maximum duration, in milliseconds, a request may take before it
    /// is considered timed out.
    pub timeout_ms: u64,

    /// Number of retries attempted before giving up.
    pub max_retries: u32,
}
```

Don't mix the two: if a field has a `///`, the blank line after it applies
regardless of whether the following field has one too.

### Enums

Same logic: bare variants packed tightly, variants with `#[error(...)]` or
`///` separated by a blank line:

```rust
// Bare, packed
pub enum Status {
    Active,
    Inactive,
    Degraded { consecutive_failures: u32 },
}

// Attribute on each variant -> blank line between each
#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("not found: {0}")]
    NotFound(String),

    #[cfg(feature = "sqlite")]
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
}
```

### Methods in an `impl` block

Always a blank line between two methods, **regardless** of whether they
have doc comments:

```rust
impl RetryPolicy {
    pub fn delay_ms(mut self, v: u64) -> Self {
        self.delay_ms = v;
        self
    }

    pub fn max_retries(mut self, v: u32) -> Self {
        self.max_retries = v;
        self
    }
}
```

### Function bodies

A short `//` comment, in the imperative, introduces each logical step; a
blank line separates the steps from each other:

```rust
// Try to get the client
let client = self.pool.get(&addr)?;

// Create the request
let request = build_request(payload);

// Send request and get response
let response = client.call(request).await?;
```

Some files spell this out explicitly with `// Step 1: …`, `// Step 2: …`.

A blank line also generally precedes the final `Ok(())` / return value
when it follows a multi-line block.

### Match arms

Arms with a non-trivial `{ … }` body are separated by a blank line; arms
with a single-expression body (one line) stay packed:

```rust
match event {
    Event::Put(kv) => {
        let key = kv.key();
        registry.insert(key, kv.value());
    }

    Event::Delete(kv) => {
        registry.remove(kv.key());
    }

    Event::Idle => continue,
}
```

### What never moves

- Never a blank line right after an opening `{` nor right before a closing
  `}`.
- `use` groups (see §2) follow their own rule, independent of doc
  comments.

## 2. `use` organization

Groups separated by a blank line, in a fixed order, alphabetical within
each group:

1. `std::*`
2. external crates (alphabetical)
3. sibling workspace crates (an internal `utils` crate, etc.) — optional
   group
4. local `crate::*`

```rust
use std::fmt::Display;
use std::hash::Hash;

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use utils::net::tcp::test_connection;

use crate::error::Error;
```

An isolated, one-off call to a function from a sibling crate can use its
full path without a dedicated `use` (`utils::uuid::new_v7()`); a repeated
usage gets its own `use`. This is a case-by-case call, not a strict rule.

## 3. Error handling

Each crate/module has its own error module with this skeleton:

```rust
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("not found: {0}")]
    NotFound(String),
}
```

- Variants sorted alphabetically, separated by a blank line (§1).
- `#[error(transparent)]` + `#[from]` for anything that's a plain
  pass-through of an external error.
- `#[error("message {0}")]` with an explicit message for domain errors.
- A conditional variant carries its `#[cfg(feature = "...")]` right above
  `#[error(...)]`.

## 4. Doc comments

- **Always documented**: public structs, public enums and their variants,
  public fields (especially once the struct already has at least one
  documented field — see §1), public traits and their methods,
  non-trivial public functions.
- **Never documented**: trivial impls of `Display::fmt`,
  `FromStr::from_str` and other std traits on simple wrappers — their
  behavior is obvious from the signature.
- Builder setters follow the same template throughout the codebase:

  ```rust
  /// Sets the port.
  ///
  /// # Arguments
  /// * `port` - The port to bind to.
  ///
  /// # Returns
  /// A new `ServerConfig` instance.
  ```

- Constructors of non-trivial public types: doc + an `# Example` section
  with a runnable code block.
- Trait doc: free-form prose, no `# Arguments`/`# Returns` sections.

## 5. Module organization

- Flat files by default; a `mod.rs` only when there's an actual
  subdirectory.
- In `lib.rs`/`mod.rs`: unconditional `mod ...;` declarations first, then
  `pub use ...;` / `pub(crate) use ...;`, then one `cfg_if::cfg_if!` block
  per optional feature, grouping its `mod` and its `pub use`:

  ```rust
  mod client;
  mod error;

  pub use client::Client;
  pub use error::{Error, Result};

  cfg_if::cfg_if! {
      if #[cfg(feature = "tls")] {
          mod tls;
          pub use tls::TlsConfig;
      }
  }
  ```

- Plain `#[cfg(feature = "...")]`, without `cfg_if!`, when it's a single
  isolated item (a config field, an error variant) rather than a module +
  re-export pair.
- Visibility: types internal to a module (e.g. an internal `Entry` or
  `State` struct) are never `pub`. A helper shared across several
  submodules but not exposed in the crate's public API goes through
  `pub(crate) use` at the `mod.rs` level, even if the type itself is `pub`
  within its own file.

## 6. Builder pattern, generics, feature flags

Two builder variants, depending on whether validation is needed:

- **Separate builder** (`XBuilder` + `X::builder()`) when `build()` needs
  to validate invariants and return a `Result<X>` — e.g. a builder that
  rejects an empty `name`.
- **Self-hosted builder** (the type itself exposes `new()` + fluent
  setters that consume and return `Self`) when there's nothing to
  validate — e.g. `Config::new().port(8080).timeout_ms(500)`. This is the
  common case for simple configuration/option types.

Setters are named exactly like the field they modify, never with a
`with_`/`set_` prefix (`.port(...)`, `.timeout_ms(...)`).

Generics: the `where` clause always sits on its own lines, even for a
single bound — never an inline bound in the angle brackets:

```rust
struct Wrapper<T>
where
    T: Clone,
{
    // ...
}
```

## 7. Naming

- Types and traits: `PascalCase`. Functions, variables, modules:
  `snake_case`. Constants: `SCREAMING_SNAKE_CASE`.
- `#[derive(...)]` order is fixed and repeated verbatim throughout the
  code: std traits first, in a fixed order, then serde traits last:

  ```rust
  #[derive(Clone, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
  ```

## 8. Functions length

- Functions should not be too long. The only exception is for matches with
  a lot of elements. For the rest, functions must be splitted in smaller
  entities, call utils functions, etc.

## 9. Other conventions

- Dead or pending code isn't deleted but left visible with a
  `// TODO: ...` marker, including whole blocks commented out for a
  future feature, kept until it's clearly safe to delete.
- A small internal helper is reused through composition rather than
  duplicated across similar implementations — e.g. a shared formatting
  helper used by several `Display` implementations to align key/value
  pairs, instead of reimplementing the same alignment logic in each one.
