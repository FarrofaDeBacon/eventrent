pub const PERM_USER_READ: &str = "user:read";
pub const PERM_USER_MANAGE: &str = "user:manage";
pub const PERM_ROLE_READ: &str = "role:read";
pub const PERM_ROLE_MANAGE: &str = "role:manage";
pub const PERM_BRANDING_READ: &str = "branding:read";
pub const PERM_BRANDING_MANAGE: &str = "branding:manage";
pub const PERM_CUSTOMER_READ: &str = "customer:read";
pub const PERM_CUSTOMER_MANAGE: &str = "customer:manage";
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
            PERM_QUOTE_READ,
            PERM_RESERVATION_READ,
            PERM_FINANCE_READ,
            PERM_FINANCE_MANAGE,
            PERM_AUDIT_READ,
            PERM_BRANDING_READ,
        ],
    },
];
