//! Read-only Boolean catalog projections for operator-facing consumers.
//! None of these decoded values grants execution, statistical or admission authority.

pub use crate::candidate_universe::boolean_candidate_v1::grid_context::GridContext;
pub use crate::candidate_universe::boolean_candidate_v1::reader::{Coordinate, Reader, Session};
pub use runner::excursion::Side;
pub use runner::exit_grid_policy::{
    ExecutionRefusalBitsV1, ExitGridSelectorV1, ForcedStopV1, RangeResolutionV1,
};
pub use runner::grid::{Cell, TradeRow};
pub use runner::research_family::ResearchFamilyV1;
