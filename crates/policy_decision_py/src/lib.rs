//! PyO3 binding for the `policy_decision` PDP. The only crate that touches FFI; the
//! core stays infrastructure-free (ADR-0011). It exposes one function over a wire
//! (`wire`), which the Python host wraps in a Pythonic API. The compiled module is the
//! private `agent_action_classifier._core` (see pyproject `[tool.maturin] module-name`).

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

mod wire;

/// Decide a verdict from a JSON action, Cedar policy source, Cedar entity JSON, and a
/// JSON context, returning a JSON decision. A parse failure surfaces as `ValueError`.
#[pyfunction]
fn decide_json(
    action_json: &str,
    policy_cedar: &str,
    entities_json: &str,
    context_json: &str,
) -> PyResult<String> {
    wire::decide_json(action_json, policy_cedar, entities_json, context_json)
        .map_err(|err| PyValueError::new_err(err.to_string()))
}

#[pymodule]
fn _core(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(decide_json, module)?)?;
    Ok(())
}
