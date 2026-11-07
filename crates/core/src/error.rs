use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum CoreError {
    #[error("Erro de banco de dados: {0}")]
    Database(String),

    #[error("Erro de migração: {0}")]
    Migration(String),

    #[error("Erro de validação: {0}")]
    Validation(String),

    #[error("Conflito de registro: {0}")]
    Conflict(String),

    #[error("Conflito transitório de concorrência: {0}")]
    TransientConflict(String),

    #[error("Registro não encontrado: {0}")]
    NotFound(String),

    #[error("E-mail ou senha inválidos.")]
    InvalidCredentials,

    #[error("Conta temporariamente bloqueada até {locked_until}")]
    AccountLocked { locked_until: String },

    #[error("Não autenticado: {0}")]
    Unauthorized(String),

    #[error("Permissão negada: requer '{0}'")]
    Forbidden(String),

    #[error("Erro interno: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;
