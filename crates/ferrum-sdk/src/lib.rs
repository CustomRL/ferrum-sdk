//! Write plugins for the [Ferrum](https://github.com/CustomRL/Ferrum) editor.
//!
//! A Ferrum plugin is a WebAssembly component. You compile a Rust crate for
//! `wasm32-wasip2`, the editor loads it, and the two talk through a WIT
//! contract called `ferrum:plugin`. This crate is the guest side: the code
//! generation, and the plumbing you should not have to think about.
//!
//! # What a plugin can do
//!
//! Exactly what it declares, and nothing else. Ferrum's capability model is not
//! a permission prompt at call time — a capability *is* an import, the editor
//! builds a linker per plugin containing only what was granted, and an
//! ungranted capability is an import nothing defines, so the plugin does not
//! start rather than failing halfway through.
//!
//! The practical consequence is a good one: **reaching for something you did
//! not declare is a compile error in your own crate**, at your desk, rather
//! than an install failure on somebody else's machine. That is why you write
//! your own world naming what you need, instead of importing one large surface.
//!
//! # Getting started
//!
//! `Cargo.toml`:
//!
//! ```toml
//! [lib]
//! crate-type = ["cdylib"]
//!
//! [dependencies]
//! ferrum-sdk = "0.1"
//!
//! [build-dependencies]
//! ferrum-sdk-build = "0.1"
//! ```
//!
//! `build.rs`:
//!
//! ```text
//! // build.rs
//! fn main() {
//!     ferrum_sdk_build::vendor();
//! }
//! ```
//!
//! `wit/world.wit`, naming what you need:
//!
//! ```text
//! package acme:hello@0.1.0;
//!
//! world hello {
//!   include ferrum:plugin/plugin-base@1.0.0;
//!   include ferrum:plugin/cap-status-item@1.0.0;
//! }
//! ```
//!
//! and `src/lib.rs`:
//!
//! ```ignore
//! ferrum_sdk::generate!("hello");
//!
//! struct Hello;
//!
//! impl exports::ferrum::plugin::lifecycle::Guest for Hello {
//!     async fn activate(
//!         _ctx: exports::ferrum::plugin::lifecycle::Context,
//!     ) -> Result<(), ferrum::plugin::types::PluginError> {
//!         Ok(())
//!     }
//!     async fn deactivate() {}
//! }
//!
//! export!(Hello);
//! ```
//!
//! Then `cargo build --target wasm32-wasip2 --release`. The `.wasm` under
//! `target/wasm32-wasip2/release/` is your plugin.
//!
//! # What ships beside it
//!
//! A `ferrum.toml` manifest saying who you are, when the editor should wake
//! you, and which capabilities you want. The editor derives what your component
//! *actually* imports and refuses the install if the two disagree, so a
//! manifest cannot promise less than the code takes.

pub use wit_bindgen;

/// Generate the bindings for your world.
///
/// Wraps [`wit_bindgen::generate!`] with the options a Ferrum plugin always
/// needs, so the three that are easy to get wrong are not yours to get wrong:
///
/// - **`generate_all`**, without which the shared `ferrum:plugin/types`
///   interface is a compile error about a missing `with` mapping rather than a
///   generated module. Every plugin hits this on its first build.
/// - **`path`**, pointing at what `ferrum_sdk_build::vendor` laid down.
/// - **`runtime_path`**, so the generated code finds its runtime through this
///   crate. Without it a plugin needs `wit-bindgen` as a direct dependency of
///   its own, which is a second version to keep in step with this one.
///
/// Takes the name of the world in your `wit/world.wit`.
#[macro_export]
macro_rules! generate {
    ($world:literal) => {
        $crate::wit_bindgen::generate!({
            world: $world,
            path: env!("FERRUM_WIT_DIR"),
            generate_all,
            // The generator picks async up from the WIT itself — an `async
            // func` becomes an `async fn`. There is deliberately no option
            // here: the host side needs to be told, the guest side does not,
            // and an option that did nothing would be one more thing to get
            // wrong.
            runtime_path: "::ferrum_sdk::wit_bindgen::rt",
        });
    };
}
