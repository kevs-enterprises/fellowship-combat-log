//! PyO3 bindings for `fellowship_combat_log`, packaged as a native Python extension module.
//!
//! Every export hands back a plain Python `dict`/`list`/`str`, built by converting the real
//! decoded value into a mirror type (see `bridge`) and serializing that through `pythonize`. The
//! precise shape of that `dict` is documented as a `TypedDict`/`Literal` union in
//! `python/fellowship_combat_log/__init__.pyi`.

mod bridge;

use bridge::{CombatantJson, EventJson, describe_parse_error, to_python};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

// The `#[pymodule]` function below is itself named `fellowship_combat_log`, which shadows the
// crate name in this scope — so the crate has to be named with a leading `::` to stay
// unambiguous.
use ::fellowship_combat_log::combatants::list_combatants as list_combatants_rs;
use ::fellowship_combat_log::parse::parse_line as parse_line_rs;

/// Decode one v8 combat-log line into the same JSON-shaped `dict` every language binding
/// produces.
///
/// Raises `ValueError` with a clear message on a malformed line — this never returns a
/// result-union.
#[pyfunction]
fn parse_line<'py>(py: Python<'py>, seq: u32, line: &str) -> PyResult<Bound<'py, PyAny>> {
    let event = parse_line_rs(seq, line)
        .map_err(|error| PyValueError::new_err(describe_parse_error(seq, &error)))?;
    let mirror: EventJson = (&event).into();
    to_python(py, &mirror)
}

/// Every combatant `log` mentions, in first-seen order, each carrying its latest gear snapshot.
///
/// This never fails: a malformed or unparseable line is skipped rather than aborting the scan,
/// exactly like the underlying Rust scan it wraps.
#[pyfunction]
fn list_combatants<'py>(py: Python<'py>, log: &str) -> PyResult<Bound<'py, PyAny>> {
    let combatants: Vec<CombatantJson> = list_combatants_rs(log)
        .iter()
        .map(CombatantJson::from)
        .collect();
    to_python(py, &combatants)
}

/// This decoder's version, for a consumer that records which decoder produced a result and wants
/// to detect staleness after an upgrade.
#[pyfunction]
fn version() -> &'static str {
    ::fellowship_combat_log::version()
}

/// A native Python module implemented in Rust: `fellowship_combat_log`.
#[pymodule]
fn fellowship_combat_log(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse_line, m)?)?;
    m.add_function(wrap_pyfunction!(list_combatants, m)?)?;
    m.add_function(wrap_pyfunction!(version, m)?)?;
    Ok(())
}
