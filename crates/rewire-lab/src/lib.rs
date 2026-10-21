//! Native Go runner for PlayRewire.
mod go_native;
mod runtime;
mod sandbox;
mod workdir;

pub use go_native::GoNativeRuntime;
pub use runtime::{LabError, LanguageRuntime, Limits, RawRunResult, SourceFile};
pub use sandbox::sandbox_probe;
