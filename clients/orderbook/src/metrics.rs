//! Orderbook Prometheus metrics definition

use prometheus::{register, Counter, Gauge, PrometheusError, Registry, U64};

/// Orderbook metrics exposed through Prometheus
pub struct Metrics {
	/// Current active validator set id
	pub ob_validator_set_id: Gauge<U64>,
	/// Total number of votes sent by this node
	pub ob_votes_sent: Counter<U64>,
	/// Most recent concluded voting round
	pub ob_round_concluded: Gauge<U64>,
	/// Best block finalized by Orderbook
	pub ob_best_block: Gauge<U64>,
	/// Next block Orderbook should vote on
	pub ob_should_vote_on: Gauge<U64>,
	/// Number of sessions with lagging signed commitment on mandatory block
	pub ob_lagging_sessions: Counter<U64>,
}

impl Metrics {
	pub fn register(registry: &Registry) -> Result<Self, PrometheusError> {
		Ok(Self {
			ob_validator_set_id: register(
				Gauge::new(
					"substrate_ob_validator_set_id",
					"Current Orderbook active validator set id.",
				)?,
				registry,
			)?,
			ob_votes_sent: register(
				Counter::new("substrate_ob_votes_sent", "Number of votes sent by this node")?,
				registry,
			)?,
			ob_round_concluded: register(
				Gauge::new(
					"substrate_ob_round_concluded",
					"Voting round, that has been concluded",
				)?,
				registry,
			)?,
			ob_best_block: register(
				Gauge::new("substrate_ob_best_block", "Best block finalized by Orderbook")?,
				registry,
			)?,
			ob_should_vote_on: register(
				Gauge::new("substrate_ob_should_vote_on", "Next block, Orderbook should vote on")?,
				registry,
			)?,
			ob_lagging_sessions: register(
				Counter::new(
					"substrate_ob_lagging_sessions",
					"Number of sessions with lagging signed commitment on mandatory block",
				)?,
				registry,
			)?,
		})
	}
}

// Note: we use the `format` macro to convert an expr into a `u64`. This will fail,
// if expr does not derive `Display`.
#[macro_export]
macro_rules! metric_set {
	($self:ident, $m:ident, $v:expr) => {{
		let val: u64 = format!("{}", $v).parse().unwrap();

		if let Some(metrics) = $self.metrics.as_ref() {
			metrics.$m.set(val);
		}
	}};
}

#[macro_export]
macro_rules! metric_inc {
	($self:ident, $m:ident) => {{
		if let Some(metrics) = $self.metrics.as_ref() {
			metrics.$m.inc();
		}
	}};
}

#[cfg(test)]
#[macro_export]
macro_rules! metric_get {
	($self:ident, $m:ident) => {{
		$self.metrics.as_ref().map(|metrics| metrics.$m.clone())
	}};
}
