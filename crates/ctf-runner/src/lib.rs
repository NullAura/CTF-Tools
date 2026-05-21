//! External process, Python worker, and operation runner assembly lives here.

use ctf_core::{OperationRegistry, OperationRunner, Result};

pub fn default_runner() -> Result<OperationRunner> {
    let registry = OperationRegistry::load_default()?;
    let mut runner = OperationRunner::new(registry);
    ctf_codecs::register_handlers(&mut runner);
    ctf_crypto::register_handlers(&mut runner);
    Ok(runner)
}
