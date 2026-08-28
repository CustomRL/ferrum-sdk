# ferrum-sdk

Write plugins for the [Ferrum](https://github.com/CustomRL/Ferrum) editor.

A Ferrum plugin is a WebAssembly component. You write a Rust crate, compile it
for `wasm32-wasip2`, and the editor loads it. This is the guest side of the
`ferrum:plugin` contract — the world itself, the code generation, and the
plumbing you should not have to think about.

## What a plugin can do

Exactly what it declares, and nothing else.

Ferrum's capability model is not a permission prompt at call time. A capability
*is* an import: the editor builds a linker per plugin containing only what was
granted, so a capability the manifest did not declare is an import nothing
defines and the component does not start. There is no runtime check to forget.

The practical consequence is a good one. **Reaching for something you did not
declare is a compile error in your own crate**, at your desk, rather than an
install failure on somebody else's machine. That is why you write your own world
naming what you need, rather than importing one large surface.

There are seventeen capabilities. Ambient ones — `session`, `log`, `clock` —
come with `plugin-base`. The rest are asked for and consented to: reading
buffers, reading the workspace, reading files under a path scope, network fetch
to a named host list, storage, notifications, commands, edits, diagnostics,
decorations, tool integration, a status reading, the plugin's own config, and
reading how much of an AI tool's rate limit has been spent.

## Exports are optional too

`plugin-base` exports `lifecycle`, and that is the only thing you must
implement. Everything else a plugin can *provide* is its own world you opt into,
for the same reason capabilities are: a world that declared an export you did
not implement would refuse to instantiate.

```
world provides-commands         // handle commands you registered
world provides-completion       // supply completions
world provides-hover            // supply hover text
world provides-code-actions     // supply code actions
world provides-symbols          // supply document symbols
world provides-buffer-events    // observe buffer changes
world provides-workspace-events // observe workspace changes
world provides-refresh          // recompute what you display, when asked
```

`provides-refresh` is worth singling out if you show anything derived from data
that moves. Without it `activate` is the only call into your plugin that will
ever happen, so whatever you displayed is frozen at editor start. Implement
`refresher::refresh` and the host calls it periodically, and again the moment a
user opens your reading:

```rust
impl exports::ferrum::plugin::refresher::Guest for Hello {
    async fn refresh() { /* recompute and re-`set` your status item */ }
}
```

The host chooses when. There is no timer in this world, deliberately — a plugin
that could schedule its own work could spend the editor's time without anybody
having granted it anything.

## Getting started

```toml
# Cargo.toml
[lib]
crate-type = ["cdylib"]

[dependencies]
ferrum-sdk = "0.1"

[build-dependencies]
ferrum-sdk-build = "0.1"
```

```rust
// build.rs
fn main() {
    ferrum_sdk_build::vendor();
}
```

```wit
// wit/world.wit
package acme:hello@0.1.0;

world hello {
  include ferrum:plugin/plugin-base@1.0.0;
  include ferrum:plugin/cap-status-item@1.0.0;
}
```

```rust
// src/lib.rs
ferrum_sdk::generate!("hello");

use exports::ferrum::plugin::lifecycle::{Context, Guest};
use ferrum::plugin::types::PluginError;

struct Hello;

impl Guest for Hello {
    async fn activate(ctx: Context) -> Result<(), PluginError> {
        ferrum::plugin::log::write(
            ferrum::plugin::log::Level::Info,
            &format!("hello: {:?}", ctx.reason),
        );
        Ok(())
    }

    async fn deactivate() {}
}

export!(Hello);
```

```console
$ rustup target add wasm32-wasip2
$ cargo build --target wasm32-wasip2 --release
```

The `.wasm` under `target/wasm32-wasip2/release/` is your plugin.

`examples/minimal` is exactly the above, and it is the first thing to build when
something stops working: if it compiles, the toolchain, the vendored world and
the code generation are all fine.

## The manifest

A `ferrum.toml` ships beside the component:

```toml
[plugin]
name = "@acme/hello"
version = "0.1.0"
world = "1.0.0"
description = "Says hello."

[activation]
explicit = true

[[capability]]
grant = "status-item"

[[setting]]
key = "greeting"
label = "Greeting"
help = "What to say."
kind = "text"
default = "hello"
```

The editor decodes what your component *actually* imports and refuses the
install if that disagrees with what you declared — so a manifest cannot promise
less than the code takes. Settings you declare are rendered by the editor's own
settings screen and handed back to you, already validated, through
`config-read`.

## Everything is async

The world is async in its first version. `stream<T>`, `future<T>` and `async
func` are Canonical ABI types, so they could never have been added later without
recompiling every published plugin. `lifecycle::activate` is an `async fn`; so
is most of what you will call.

`activate` must reach its first await inside one execution slice. Obtaining
handles and starting stream loops belongs there; indexing a workspace does not.

## What this is not

There is no way to draw. No webviews, no HTML, no widget tree, no canvas — not
"not yet", but by design. `notify` says what needs saying and the host decides
how it appears; `status-item` puts a label and a bar in the chrome and the host
owns every pixel of it. A plugin supplies data and the editor renders it.

That is a real constraint and it rules out a class of extension that exists
elsewhere. It is also what lets the editor stay a native renderer with a frame
budget rather than an application that ships a browser.

## Versioning

`WORLD_VERSION` is `1.0.0`, not `0.1.0`, deliberately: under the pre-1.0 semver
rules that `wasm-tools` and wasmtime's version-compatible linking follow,
`0.1.0` and `0.2.0` are incompatible *majors*, which would make the first
additive change to the world a flag day for every published plugin.

Adding a capability is additive — existing components do not import it, so they
are unaffected. So is adding a provider export world, and adding a function to
an interface: a component that does not import or export the new thing links
exactly as it did.

Adding a *field to a record* is not. The Canonical ABI is structural, so every
record, enum and variant in `types` is frozen for the life of major version 1,
and so is every record in an interface that has shipped.

"Has shipped" is the operative phrase, and it is doing real work rather than
softening the rule. `cap-usage-read` gained a `measured-minutes-ago` field on
its `limits` record after this crate existed but before this crate was ever
tagged or published — so the set of components that would have broken was
empty. That window is now closed. It closes for good the moment a version of
this crate is published, because from then on the components are somebody
else's and the editor has no way to recompile them.

## Licence

`MIT OR Apache-2.0`, at your option.

The editor is GPL-3.0-or-later. The SDK deliberately is not: an SDK under the
same licence would make every plugin that links it GPL too, which decides for
you what you are allowed to write. That is not the SDK's decision to make.
