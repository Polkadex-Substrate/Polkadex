//! Orderbook Prometheus metrics definition

use prometheus::{register, Counter, Gauge, PrometheusError, Registry, U64};

/// Orderbook metrics exposed through Prometheus
pub struct Metrics {
	/// Last processed state id
	pub ob_state_id: Gauge<U64>,
	/// Total number of ob messages sent by this node
	pub ob_messages_sent: Counter<U64>,
	/// Total number of ob messages recvd by this node
	pub ob_messages_recv: Counter<U64>,
}

impl Metrics {
	pub fn register(registry: &Registry) -> Result<Self, PrometheusError> {
		Ok(Self {
			ob_state_id: register(
				Gauge::new("polkadex_ob_state_id", "Last processed state id by Orderbook")?,
				registry,
			)?,
			ob_messages_sent: register(
				Counter::new("polkadex_ob_messages_sent", "Number of messages sent by this node")?,
				registry,
			)?,
			ob_messages_recv: register(
				Counter::new(
					"polkadex_ob_messages_recv",
					"Number of messages received by this node",
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
