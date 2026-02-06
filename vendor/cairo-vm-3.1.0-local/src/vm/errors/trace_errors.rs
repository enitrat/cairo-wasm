use thiserror::Error;

use crate::vm::errors::memory_errors::MemoryError;

#[derive(Debug, PartialEq, Error)]
pub enum TraceError {
    #[error("Trace is not enabled for this run")]
    TraceNotEnabled,
    #[error("Trace is already relocated")]
    AlreadyRelocated,
    #[error("No relocation found for execution segment")]
    NoRelocationFound,
    #[error(transparent)]
    MemoryError(#[from] MemoryError),
    #[error("Trace not relocated")]
    TraceNotRelocated,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // Test to catch possible enum size regressions
    fn test_trace_error_size() {
        let size = crate::stdlib::mem::size_of::<TraceError>();
        assert!(size <= 24, "{size}")
    }
}
