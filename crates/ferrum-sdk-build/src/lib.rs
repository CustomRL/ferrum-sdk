//! Put the `ferrum:plugin` world where `wit_bindgen::generate!` can read it.
//!
//! Call [`vendor`] from your plugin's `build.rs` and forget about it:
//!
//! ```text
//! // build.rs
//! fn main() {
//!     ferrum_sdk_build::vendor();
//! }
//! ```
//!
//! # Why this exists at all
//!
//! `wit_bindgen::generate!` reads WIT from a directory on disk, not from a
//! crate. Without this, every plugin author copies twenty-five files into their
//! own repository — once — and the copy is never updated again, so the first
//! additive change to the world turns into a support question rather than a
//! `cargo update`.
//!
//! The files are carried as bytes in this crate rather than read from a path
//! baked in at compile time. A path works right up until somebody clears the
//! registry cache or vendors their dependencies for an offline build, and then
//! a plugin fails to compile with an error naming a directory the author has
//! never heard of.

include!(concat!(env!("OUT_DIR"), "/world.rs"));

use std::path::{Path, PathBuf};

/// Write the world into `OUT_DIR` and point the `generate!` macro at it.
///
/// Sets `FERRUM_WIT_DIR` for the crate being built, which is what
/// `ferrum_sdk::generate!` reads. Your own `wit/world.wit` is copied in beside
/// it, so one directory holds both your world and the package it includes.
///
/// # Panics
///
/// If `wit/world.wit` is missing, or the files cannot be written. Both are
/// build-time faults with nothing useful to recover to: a plugin with no world
/// has nothing to generate, and a build script that cannot write to `OUT_DIR`
/// has a broken toolchain rather than a broken plugin.
pub fn vendor() {
    vendor_from(Path::new("wit"));
}

/// [`vendor`], with your world somewhere other than `wit/`.
pub fn vendor_from(world_dir: &Path) {
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR is set by cargo"));
    let root = out.join("ferrum-wit");
    let deps = root.join("deps").join("ferrum-plugin");

    std::fs::create_dir_all(&deps).expect("create the vendored world directory");
    for (name, contents) in WORLD {
        write_if_changed(&deps.join(name), contents);
    }

    // The author's own world, beside the package it includes. `generate!` takes
    // one path, so both have to live under it.
    let mine = world_dir.join("world.wit");
    let source = std::fs::read_to_string(&mine).unwrap_or_else(|why| {
        panic!(
            "{} could not be read ({why}). A plugin declares its own world \
             naming the capabilities it needs — see the ferrum-sdk docs.",
            mine.display()
        )
    });
    write_if_changed(&root.join("world.wit"), &source);

    println!("cargo:rerun-if-changed={}", mine.display());
    println!("cargo:rustc-env=FERRUM_WIT_DIR={}", root.display());
}

/// Write only when the contents differ.
///
/// `OUT_DIR` contents feed a `rerun-if-changed` graph, and rewriting identical
/// files with new timestamps makes every build look dirty to whatever is
/// watching — which for a plugin author is their editor rebuilding on every
/// save of an unrelated file.
fn write_if_changed(path: &Path, contents: &str) {
    if std::fs::read_to_string(path).is_ok_and(|existing| existing == contents) {
        return;
    }
    std::fs::write(path, contents).unwrap_or_else(|why| {
        panic!("could not write {} ({why})", path.display());
    });
}

/// Which version of the world this crate carries.
///
/// Worth reporting in a plugin's own diagnostics: an author on an older SDK is
/// building against an older contract, and "which world" is the first question
/// when something does not link.
pub const WORLD_VERSION: &str = "1.0.0";

/// How many files the world is made of.
pub fn world_files() -> usize {
    WORLD.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_world_came_along_for_the_ride() {
        // The whole reason this crate exists. An empty list means the build
        // script found nothing and every plugin would fail to generate.
        assert!(world_files() >= 20, "only {} files", world_files());
    }

    #[test]
    fn the_shared_types_interface_is_present() {
        // Everything transitively depends on it, so its absence is the one
        // missing file that breaks every world rather than one of them.
        assert!(WORLD.iter().any(|(name, _)| *name == "types.wit"));
        assert!(WORLD.iter().any(|(name, _)| *name == "worlds.wit"));
    }

    #[test]
    fn every_file_names_the_package() {
        // A file that lost its `package` line would parse as belonging to
        // whatever came before it, which is the kind of error that surfaces as
        // a baffling type mismatch three interfaces away.
        for (name, contents) in WORLD {
            assert!(
                contents.contains("package ferrum:plugin@"),
                "{name} does not declare the package"
            );
        }
    }

    #[test]
    fn nothing_is_empty() {
        for (name, contents) in WORLD {
            assert!(contents.len() > 40, "{name} is {} bytes", contents.len());
        }
    }
}
