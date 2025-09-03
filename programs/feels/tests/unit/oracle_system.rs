// TODO: Reimplement oracle/twap tests against the new `state::twap_oracle`
// and market data source update path. Cover:
// - Observation ring buffer behavior and size
// - Freshness bounds and rate-of-change caps on updates
// - TWAP computation and variance tracking in the unified numeraire
