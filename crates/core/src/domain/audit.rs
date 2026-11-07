use serde::{Deserialize, Serialize};

/// Registro de auditoria append-only.
/// Não possui `updated_at` nem `deleted_at` porque registros de auditoria
/// nunca devem ser alterados ou excluídos (imutabilidade por design).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditLogEntry {
    pub id: String,
    pub tenant_id: String,
    pub actor_user_id: Option<String>,
    pub action: String,
    pub entity_type: String,
    pub entity_id: String,
    pub summary: String,
    pub created_at: String,
    pub created_by: Option<String>,
}
