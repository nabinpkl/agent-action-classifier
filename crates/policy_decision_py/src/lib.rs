//! PyO3 binding for the `policy_decision` PDP. The only crate that touches FFI; the
//! core stays infrastructure-free (ADR-0011). It exposes one function over a wire
//! (`wire`), which the Python host wraps in a Pythonic API. The compiled module is the
//! private `agent_action_classifier._core` (see pyproject `[tool.maturin] module-name`).

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use policy_decision::policy::Policy;

mod wire;

/// A compiled org policy: the Cedar schema + policies + entity store parsed and validated
/// once into an in-memory handle, then reused for many decisions (ADR-0019, the parse-once
/// lifecycle). Construction is the compile step (paid once per policy load); `decide` is
/// the hot path and does no policy parsing.
#[pyclass]
struct CompiledPolicy {
    policy: Policy,
}

#[pymethods]
impl CompiledPolicy {
    /// Compile the three Cedar artifacts into the handle: `schema_cedar` (the contract),
    /// `policy_cedar` (the rules), and `entities_json` (the entity store). They are
    /// validated as a unit; a parse or schema-validation failure surfaces as `ValueError`.
    #[new]
    fn new(schema_cedar: &str, policy_cedar: &str, entities_json: &str) -> PyResult<Self> {
        let policy = Policy::from_sources(schema_cedar, policy_cedar, entities_json)
            .map_err(|err| PyValueError::new_err(err.to_string()))?;
        Ok(Self { policy })
    }

    /// Decide a verdict for one JSON action under a JSON context, returning a JSON decision.
    /// A malformed action or context surfaces as `ValueError`.
    fn decide(&self, action_json: &str, context_json: &str) -> PyResult<String> {
        wire::decide(&self.policy, action_json, context_json)
            .map_err(|err| PyValueError::new_err(err.to_string()))
    }
}

#[pymodule]
fn _core(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<CompiledPolicy>()?;
    Ok(())
}
