// This file is part of Polkadex.
//
// Copyright (c) 2023 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Orderbook Prometheus metrics definition

use prometheus::{register, Counter, Gauge, PrometheusError, Registry, U64};

/// Orderbook metrics exposed through Prometheus
pub struct Metrics {
	/// Last processed snapshot id
	pub ob_snapshot_id: Gauge<U64>,
	/// Last processed worker nonce
	pub ob_worker_nonce: Gauge<U64>,
	/// Total number of ob messages sent by this node
	pub ob_messages_sent: Counter<U64>,
	/// Total number of ob messages recvd by this node
	pub ob_messages_recv: Counter<U64>,
	/// Total data sent out by ob worker
	pub ob_data_sent: Gauge<U64>,
	/// Total data recv by ob worker
	pub ob_data_recv: Gauge<U64>,
}

impl Metrics {
	pub fn register(registry: &Registry) -> Result<Self, PrometheusError> {
		Ok(Self {
			ob_snapshot_id: register(
				Gauge::new("polkadex_ob_state_id", "Last processed state id by Orderbook")?,
				registry,
			)?,
			ob_worker_nonce: register(
				Gauge::new("polkadex_ob_worker_nonce", "Last processed worker nonce by Orderbook")?,
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
			ob_data_sent: register(
				Gauge::new("polkadex_ob_data_sent", "Total Data sent by orderbook worker")?,
				registry,
			)?,
			ob_data_recv: register(
				Gauge::new("polkadex_ob_data_recv", "Total Data received by orderbook worker")?,
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

#[macro_export]
macro_rules! metric_add {
	($self:ident, $m:ident, $v:expr) => {{
		let val: u64 = format!("{}", $v).parse().unwrap();

		if let Some(metrics) = $self.metrics.as_ref() {
			metrics.$m.add(val);
		}
	}};
}
