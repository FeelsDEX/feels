/// Unified rebase system for yield distribution across different dimensions.
/// Implements strategies for lending, leverage, funding, and weight rebasing.

pub mod rebase;
pub mod rebase_funding;
pub mod rebase_lending;
pub mod rebase_leverage;
pub mod weight_rebase;

// Re-export the unified framework
pub use rebase::{
    RebaseStrategy, RebaseFactors, RebaseState, RebaseParams,
    DomainParams, DomainWeights, RebaseExecutor,
};

// Re-export specific strategies
pub use rebase_funding::{
    FundingRebaseFactors, FundingRebaseStrategy, calculate_funding_rebase,
};
pub use rebase_lending::{
    LendingRebaseFactors, LendingRebaseStrategy, calculate_lending_rebase,
};
pub use rebase_leverage::{
    LeverageRebaseFactors, LeverageRebaseStrategy, calculate_leverage_rebase,
};
pub use weight_rebase::{
    WeightRebaseFactors, WeightRebaseStrategy, calculate_weight_rebase,
};