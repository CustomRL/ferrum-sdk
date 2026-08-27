//! The smallest plugin that is a plugin.
//!
//! It activates and does nothing. That is not a joke: `lifecycle` is the only
//! mandatory export, so this is the complete set of obligations, and if this
//! builds then the toolchain, the vendored world and the code generation are
//! all working. Start here when something stops compiling.

ferrum_sdk::generate!("minimal");

use exports::ferrum::plugin::lifecycle::{Context, Guest};
use ferrum::plugin::types::PluginError;

struct Minimal;

impl Guest for Minimal {
    async fn activate(ctx: Context) -> Result<(), PluginError> {
        // `log` is ambient — every plugin has it, so there is nothing to
        // declare and nothing to consent to.
        ferrum::plugin::log::write(
            ferrum::plugin::log::Level::Info,
            &format!("minimal activated: {:?}", ctx.reason),
        );
        Ok(())
    }

    async fn deactivate() {}
}

export!(Minimal);
