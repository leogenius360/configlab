//! Source-neutral configuration resolution.

mod context;
mod contribution;
mod diagnostics;
mod engine;
mod error;
mod input;
mod layer;
mod merge;
mod path;
mod peer;
mod provenance;
mod verification;

pub use context::{Context, ResolutionRequest, Selector};
pub use engine::Resolver;
pub use error::ResolveError;
pub use input::{InputTarget, LogicalInput, Operation, Origin};
pub use layer::LayerOrder;
pub use provenance::{
    EffectiveConfiguration, ProvenanceAction, ProvenanceEntry, RemovalKind, ResolutionInformation,
    provenance_child_path,
};
pub use verification::{VerificationCheck, VerificationReport, VerificationResult};

// Internal path helpers shared by resolution submodules.
pub(crate) use path::{get_path, get_path_mut, remove_path, set_existing_path, set_path};
