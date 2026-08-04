pub mod builtin;
pub mod codex;
pub mod hermes;
pub mod openclaw;
mod sdk_backed;
pub mod zeroclaw;

pub use builtin::BuiltinPlugins;
pub use codex::{CodexPlugin, CodexProvider};
pub use hermes::{HermesPlugin, HermesProvider};
pub use openclaw::{OpenClawPlugin, OpenClawProvider};
pub use sdk_backed::{
    ClaudeCodePlugin, ClaudeCodeProvider, GeminiCliPlugin, GeminiCliProvider, OpenCodePlugin,
    OpenCodeProvider,
};
pub use zeroclaw::{ZeroCloudPlugin, ZeroCloudProvider};
