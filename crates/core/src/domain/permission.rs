pub const PERM_USER_READ: &str = "user:read";
pub const PERM_USER_MANAGE: &str = "user:manage";
pub const PERM_ROLE_READ: &str = "role:read";
pub const PERM_ROLE_MANAGE: &str = "role:manage";
pub const PERM_BRANDING_READ: &str = "branding:read";
pub const PERM_BRANDING_MANAGE: &str = "branding:manage";
pub const PERM_CUSTOMER_READ: &str = "customer:read";
pub const PERM_CUSTOMER_MANAGE: &str = "customer:manage";
pub const PERM_SUPPLIER_READ: &str = "supplier:read";
pub const PERM_SUPPLIER_MANAGE: &str = "supplier:manage";
pub const PERM_VENUE_READ: &str = "venue:read";
pub const PERM_VENUE_MANAGE: &str = "venue:manage";
pub const PERM_CATALOG_READ: &str = "catalog:read";
pub const PERM_CATALOG_MANAGE: &str = "catalog:manage";
pub const PERM_RESERVATION_READ: &str = "reservation:read";
pub const PERM_RESERVATION_MANAGE: &str = "reservation:manage";
pub const PERM_QUOTE_READ: &str = "quote:read";
pub const PERM_QUOTE_MANAGE: &str = "quote:manage";
pub const PERM_WAREHOUSE_READ: &str = "warehouse:read";
pub const PERM_WAREHOUSE_OPERATE: &str = "warehouse:operate";
pub const PERM_FINANCE_READ: &str = "finance:read";
pub const PERM_FINANCE_MANAGE: &str = "finance:manage";
pub const PERM_AUDIT_READ: &str = "audit:read";
pub const PERM_SETTINGS_READ: &str = "settings:read";
pub const PERM_SETTINGS_MANAGE: &str = "settings:manage";
pub const PERM_BACKUP_MANAGE: &str = "backup:manage";

pub const ROLE_SLUG_OWNER: &str = "owner";
pub const ROLE_SLUG_COMMERCIAL: &str = "commercial";
pub const ROLE_SLUG_WAREHOUSE: &str = "warehouse";
pub const ROLE_SLUG_TECHNICIAN: &str = "technician";
pub const ROLE_SLUG_FINANCE: &str = "finance";

pub const ALL_PERMISSIONS: &[&str] = &[
    PERM_USER_READ,
    PERM_USER_MANAGE,
    PERM_ROLE_READ,
    PERM_ROLE_MANAGE,
    PERM_BRANDING_READ,
    PERM_BRANDING_MANAGE,
    PERM_CUSTOMER_READ,
    PERM_CUSTOMER_MANAGE,
    PERM_SUPPLIER_READ,
    PERM_SUPPLIER_MANAGE,
    PERM_VENUE_READ,
    PERM_VENUE_MANAGE,
    PERM_CATALOG_READ,
    PERM_CATALOG_MANAGE,
    PERM_RESERVATION_READ,
    PERM_RESERVATION_MANAGE,
    PERM_QUOTE_READ,
    PERM_QUOTE_MANAGE,
    PERM_WAREHOUSE_READ,
    PERM_WAREHOUSE_OPERATE,
    PERM_FINANCE_READ,
    PERM_FINANCE_MANAGE,
    PERM_AUDIT_READ,
    PERM_SETTINGS_READ,
    PERM_SETTINGS_MANAGE,
    PERM_BACKUP_MANAGE,
];

#[derive(Debug, Clone)]
pub struct SystemRoleSpec {
    pub name: &'static str,
    pub slug: &'static str,
    pub description: &'static str,
    pub permissions: &'static [&'static str],
}

pub const SYSTEM_ROLES: &[SystemRoleSpec] = &[
    SystemRoleSpec {
        name: "Proprietário",
        slug: ROLE_SLUG_OWNER,
        description:
            "Acesso total à administração, configurações, usuários, financeiro e operação.",
        permissions: ALL_PERMISSIONS,
    },
    SystemRoleSpec {
        name: "Comercial",
        slug: ROLE_SLUG_COMMERCIAL,
        description: "Gestão de clientes, orçamentos, reservas e consulta ao catálogo.",
        permissions: &[
            PERM_CUSTOMER_READ,
            PERM_CUSTOMER_MANAGE,
            PERM_SUPPLIER_READ,
            PERM_VENUE_READ,
            PERM_VENUE_MANAGE,
            PERM_CATALOG_READ,
            PERM_RESERVATION_READ,
            PERM_RESERVATION_MANAGE,
            PERM_QUOTE_READ,
            PERM_QUOTE_MANAGE,
            PERM_BRANDING_READ,
        ],
    },
    SystemRoleSpec {
        name: "Estoque",
        slug: ROLE_SLUG_WAREHOUSE,
        description: "Gestão do catálogo de equipamentos, patrimônio, separação e conferência.",
        permissions: &[
            PERM_SUPPLIER_READ,
            PERM_SUPPLIER_MANAGE,
            PERM_VENUE_READ,
            PERM_VENUE_MANAGE,
            PERM_CATALOG_READ,
            PERM_CATALOG_MANAGE,
            PERM_RESERVATION_READ,
            PERM_WAREHOUSE_READ,
            PERM_WAREHOUSE_OPERATE,
            PERM_BRANDING_READ,
        ],
    },
    SystemRoleSpec {
        name: "Técnico",
        slug: ROLE_SLUG_TECHNICIAN,
        description:
            "Operação de conferência de carga, leitura QR, manutenção e consulta de eventos.",
        permissions: &[
            PERM_VENUE_READ,
            PERM_CATALOG_READ,
            PERM_RESERVATION_READ,
            PERM_WAREHOUSE_READ,
            PERM_WAREHOUSE_OPERATE,
            PERM_BRANDING_READ,
        ],
    },
    SystemRoleSpec {
        name: "Financeiro",
        slug: ROLE_SLUG_FINANCE,
        description: "Gestão de faturamento, contas a pagar/receber, contratos e auditoria.",
        permissions: &[
            PERM_CUSTOMER_READ,
            PERM_SUPPLIER_READ,
            PERM_SUPPLIER_MANAGE,
            PERM_VENUE_READ,
            PERM_QUOTE_READ,
            PERM_RESERVATION_READ,
            PERM_FINANCE_READ,
            PERM_FINANCE_MANAGE,
            PERM_AUDIT_READ,
            PERM_BRANDING_READ,
        ],
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::path::Path;

    fn extract_resource_action_strings(text: &str) -> Vec<String> {
        let mut results = Vec::new();
        for word in text.split(|c: char| !(c.is_ascii_lowercase() || c == '_' || c == ':')) {
            if let Some((left, right)) = word.split_once(':') {
                if !left.is_empty()
                    && !right.is_empty()
                    && !right.contains(':')
                    && left.chars().all(|c| c.is_ascii_lowercase() || c == '_')
                    && right.chars().all(|c| c.is_ascii_lowercase() || c == '_')
                {
                    results.push(word.to_string());
                }
            }
        }
        results.sort();
        results.dedup();
        results
    }

    #[test]
    fn test_all_permissions_in_documentation_match_canonical_catalog() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root_dir = manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .unwrap_or(manifest_dir);
        let docs_to_check = [
            root_dir.join("docs").join("03-regras-negocio.md"),
            root_dir.join("docs").join("06-requisitos-de-produto.md"),
        ];

        let canonical_set: HashSet<&str> = ALL_PERMISSIONS.iter().copied().collect();
        let mut total_found = 0;

        for doc_path in &docs_to_check {
            assert!(
                doc_path.exists(),
                "Documento obrigatório não encontrado: {}",
                doc_path.display()
            );

            let content = std::fs::read_to_string(doc_path)
                .unwrap_or_else(|e| panic!("Falha ao ler {}: {e}", doc_path.display()));

            let extracted = extract_resource_action_strings(&content);
            for perm in extracted {
                total_found += 1;
                assert!(
                    canonical_set.contains(perm.as_str()),
                    "Permissão fictícia ou descontinuada encontrada em {}: '{}'. Deve pertencer a ALL_PERMISSIONS.",
                    doc_path.display(),
                    perm
                );
            }
        }

        assert!(
            total_found > 0,
            "O validador deve encontrar e validar as permissões na documentação"
        );
    }
}
