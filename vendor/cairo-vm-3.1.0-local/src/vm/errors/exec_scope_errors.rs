use thiserror::Error;

#[derive(Eq, Hash, PartialEq, Debug, Error)]
pub enum ExecScopeError {
    #[error("Cannot exit main scope.")]
    ExitMainScopeError,
    #[error("Every enter_scope() requires a corresponding exit_scope().")]
    NoScopeError,
}
