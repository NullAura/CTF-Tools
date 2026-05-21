//! External process, Python worker, and operation runner assembly lives here.

use ctf_core::{OperationRegistry, OperationRunner, Result};

pub fn default_runner() -> Result<OperationRunner> {
    let registry = OperationRegistry::load_default()?;
    Ok(OperationRunner::new(registry))
}
