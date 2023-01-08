// Declare an instance of the native executor named `ExecutorDispatch`. Include the wasm binary as
// the equivalent wasm code.
pub struct ExecutorDispatch;

impl sc_executor::NativeExecutionDispatch for ExecutorDispatch {
	type ExtendHostFunctions = (frame_benchmarking::benchmarking::HostFunctions, thea_primitives::thea_ext::HostFunctions);

	fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
		node_polkadex_runtime::api::dispatch(method, data)
	}

	fn native_version() -> sc_executor::NativeVersion {
		node_polkadex_runtime::native_version()
	}
}
