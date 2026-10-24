//! Native Go runner for PlayRewire.
mod go_native;
mod runtime;
mod sandbox;
mod toolchain;
mod workdir;

pub use go_native::GoNativeRuntime;
pub use runtime::{LabError, LanguageRuntime, Limits, RawRunResult, SourceFile};
pub use sandbox::{PACKAGED_BWRAP, sandbox_probe};
pub use toolchain::{
    GO_VERSION, Pinned, SHA256_LINUX_AMD64, SHA256_LINUX_ARM64, SIZE_LINUX_AMD64, SIZE_LINUX_ARM64,
    ToolchainError, ToolchainStatus, install, install_with, status, status_with_env, warm_cache,
};
