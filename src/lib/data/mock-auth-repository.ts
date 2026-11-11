import type { AuthRepository } from "./auth-repository";
import {
  DEFAULT_COMPANY_NAME,
  PERMISSIONS,
  ROLE_SLUGS,
  type AppConfig,
  type AppStatus,
  type InitialSetupInput,
  type InitialSetupResult,
  type LoginInput,
  type RecoverPasswordInput,
  type RoleWithPermissions,
  type SessionInfo,
  type TenantBranding,
  type UserWithRole,
} from "./types";

const DEFAULT_TENANT_ID = "018f0000-0000-7000-8000-000000000001";
const STORAGE_KEY = "eventrent_mock_state_v1";

export interface MockInternalUser extends UserWithRole {
  password: string;
  recoveryKey: string | null;
}

export interface MockSharedState {
  config: AppConfig;
  defaultTenantId: string;
  branding: TenantBranding;
  roles: RoleWithPermissions[];
  users: MockInternalUser[];
  session: SessionInfo | null;
}

const ALL_PERMISSIONS = Object.values(PERMISSIONS);

function createSystemRoles(tenantId: string): RoleWithPermissions[] {
  const now = new Date().toISOString();
  return [
    {
      id: "role-owner",
      tenant_id: tenantId,
      name: "Proprietário",
      slug: ROLE_SLUGS.OWNER,
      description:
        "Acesso total à administração, usuários, perfis, financeiro e operação.",
      is_system: true,
      permissions: [...ALL_PERMISSIONS],
      created_at: now,
      updated_at: now,
      deleted_at: null,
      created_by: null,
    },
    {
      id: "role-commercial",
      tenant_id: tenantId,
      name: "Comercial",
      slug: ROLE_SLUGS.COMMERCIAL,
      description:
        "Gestão de clientes, orçamentos, reservas e consulta ao catálogo.",
      is_system: true,
      permissions: [
        PERMISSIONS.BRANDING_READ,
        PERMISSIONS.CATALOG_READ,
        PERMISSIONS.CUSTOMER_READ,
        PERMISSIONS.CUSTOMER_MANAGE,
        PERMISSIONS.QUOTE_READ,
        PERMISSIONS.QUOTE_MANAGE,
        PERMISSIONS.RESERVATION_READ,
        PERMISSIONS.FINANCE_READ,
      ],
      created_at: now,
      updated_at: now,
      deleted_at: null,
      created_by: null,
    },
    {
      id: "role-warehouse",
      tenant_id: tenantId,
      name: "Estoque",
      slug: ROLE_SLUGS.WAREHOUSE,
      description:
        "Gestão de catálogo de equipamentos, números de série, kits e separação.",
      is_system: true,
      permissions: [
        PERMISSIONS.BRANDING_READ,
        PERMISSIONS.CATALOG_READ,
        PERMISSIONS.CATALOG_MANAGE,
        PERMISSIONS.WAREHOUSE_READ,
        PERMISSIONS.WAREHOUSE_OPERATE,
        PERMISSIONS.RESERVATION_READ,
      ],
      created_at: now,
      updated_at: now,
      deleted_at: null,
      created_by: null,
    },
    {
      id: "role-technician",
      tenant_id: tenantId,
      name: "Técnico",
      slug: ROLE_SLUGS.TECHNICIAN,
      description:
        "Operação de saída, retorno, checklist técnico e manutenção.",
      is_system: true,
      permissions: [
        PERMISSIONS.BRANDING_READ,
        PERMISSIONS.CATALOG_READ,
        PERMISSIONS.WAREHOUSE_READ,
        PERMISSIONS.WAREHOUSE_OPERATE,
        PERMISSIONS.RESERVATION_READ,
      ],
      created_at: now,
      updated_at: now,
      deleted_at: null,
      created_by: null,
    },
    {
      id: "role-finance",
      tenant_id: tenantId,
      name: "Financeiro",
      slug: ROLE_SLUGS.FINANCE,
      description:
        "Faturamento, contas a receber, cobranças e relatórios financeiros.",
      is_system: true,
      permissions: [
        PERMISSIONS.BRANDING_READ,
        PERMISSIONS.CUSTOMER_READ,
        PERMISSIONS.QUOTE_READ,
        PERMISSIONS.RESERVATION_READ,
        PERMISSIONS.FINANCE_READ,
        PERMISSIONS.FINANCE_MANAGE,
        PERMISSIONS.AUDIT_READ,
      ],
      created_at: now,
      updated_at: now,
      deleted_at: null,
      created_by: null,
    },
  ];
}

function createInitialMockState(): MockSharedState {
  return {
    config: {
      mode: null,
      setup_completed: false,
      inactivity_timeout_minutes: 30,
    },
    defaultTenantId: DEFAULT_TENANT_ID,
    branding: {
      id: "018f0000-0000-7000-8000-000000000002",
      tenant_id: DEFAULT_TENANT_ID,
      company_name: DEFAULT_COMPANY_NAME,
      primary_color: "#2563eb",
      secondary_color: "#475569",
      accent_color: "#7c3aed",
      logo_url: null,
      logo_dark_url: null,
      icon_url: null,
      favicon_url: null,
      created_at: "2026-01-01T00:00:00Z",
      updated_at: "2026-01-01T00:00:00Z",
      deleted_at: null,
      created_by: null,
    },
    roles: createSystemRoles(DEFAULT_TENANT_ID),
    users: [],
    session: null,
  };
}

let memoryState: MockSharedState = createInitialMockState();

export function getSharedMockState(): MockSharedState {
  if (typeof window !== "undefined" && window.sessionStorage) {
    const raw = window.sessionStorage.getItem(STORAGE_KEY);
    if (raw) {
      try {
        memoryState = JSON.parse(raw) as MockSharedState;
      } catch {
        // Ignora erro de parse e mantém memoryState
      }
    }
  }
  return memoryState;
}

export function saveSharedMockState(state: MockSharedState): void {
  memoryState = state;
  if (typeof window !== "undefined" && window.sessionStorage) {
    try {
      window.sessionStorage.setItem(STORAGE_KEY, JSON.stringify(state));
    } catch {
      // Ignora indisponibilidade de storage
    }
  }
}

export function resetSharedMockState(): void {
  memoryState = createInitialMockState();
  if (typeof window !== "undefined" && window.sessionStorage) {
    window.sessionStorage.removeItem(STORAGE_KEY);
  }
}

function generateMockRecoveryKey(): string {
  const alphabet = "23456789ABCDEFGHJKLMNPQRSTUVWXYZ";
  const groups: string[] = [];
  for (let g = 0; g < 4; g++) {
    let part = "";
    for (let i = 0; i < 4; i++) {
      const idx = Math.floor(Math.random() * alphabet.length);
      part += alphabet[idx];
    }
    groups.push(part);
  }
  return `ER-${groups.join("-")}`;
}

export class MockAuthRepository implements AuthRepository {
  async getAppStatus(): Promise<AppStatus> {
    const state = getSharedMockState();
    return {
      config: { ...state.config },
      default_tenant_id: state.defaultTenantId,
      has_owner: state.users.length > 0,
      session: state.session ? { ...state.session } : null,
      branding: { ...state.branding },
    };
  }

  async completeInitialSetup(
    input: InitialSetupInput,
  ): Promise<InitialSetupResult> {
    const state = getSharedMockState();
    if (input.mode === "cloud") {
      throw new Error(
        "O modo nuvem estará disponível em breve. Escolha 'Neste computador'.",
      );
    }
    if (state.users.length > 0) {
      throw new Error("A configuração inicial já foi concluída.");
    }
    if (input.company_name.trim().length < 2) {
      throw new Error("Informe o nome da empresa (mínimo de 2 caracteres).");
    }
    if (input.owner_name.trim().length < 2) {
      throw new Error(
        "Informe o nome do proprietário (mínimo de 2 caracteres).",
      );
    }
    if (!input.owner_email.includes("@")) {
      throw new Error("Informe um e-mail válido.");
    }
    if (input.owner_password.length < 8) {
      throw new Error("A senha deve ter pelo menos 8 caracteres.");
    }

    const ownerRole = state.roles.find((r) => r.slug === ROLE_SLUGS.OWNER)!;
    const now = new Date().toISOString();
    const recoveryKey = generateMockRecoveryKey();
    const userId = `user-owner-${Date.now()}`;

    const ownerUser: MockInternalUser = {
      id: userId,
      tenant_id: state.defaultTenantId,
      name: input.owner_name.trim(),
      email: input.owner_email.trim().toLowerCase(),
      role_id: ownerRole.id,
      role_name: ownerRole.name,
      role_slug: ownerRole.slug,
      permissions: [...ownerRole.permissions],
      active: true,
      last_login_at: now,
      failed_attempts: 0,
      locked_until: null,
      created_at: now,
      updated_at: now,
      deleted_at: null,
      created_by: null,
      password: input.owner_password,
      recoveryKey,
    };

    const session: SessionInfo = {
      user_id: ownerUser.id,
      tenant_id: state.defaultTenantId,
      name: ownerUser.name,
      email: ownerUser.email,
      role_id: ownerRole.id,
      role_name: ownerRole.name,
      role_slug: ownerRole.slug,
      permissions: [...ownerRole.permissions],
      authenticated_at: now,
      last_activity_at: now,
      inactivity_timeout_minutes: state.config.inactivity_timeout_minutes,
    };

    state.branding = {
      ...state.branding,
      company_name: input.company_name.trim(),
      updated_at: now,
    };
    state.config = {
      ...state.config,
      mode: "local",
      setup_completed: true,
    };
    state.users.push(ownerUser);
    state.session = session;
    saveSharedMockState(state);

    return {
      tenant_id: state.defaultTenantId,
      recovery_key: recoveryKey,
      session: { ...session },
      branding: { ...state.branding },
    };
  }

  async login(input: LoginInput): Promise<SessionInfo> {
    const state = getSharedMockState();
    const email = input.email.trim().toLowerCase();
    const user = state.users.find((u) => u.email === email && !u.deleted_at);

    if (!user) {
      throw new Error("E-mail ou senha inválidos.");
    }
    if (!user.active) {
      throw new Error(
        "Esta conta foi desativada. Contate o proprietário da empresa.",
      );
    }

    if (user.locked_until) {
      const untilMs = Date.parse(user.locked_until);
      const nowMs = Date.now();
      if (untilMs > nowMs) {
        const remainingSeconds = Math.ceil((untilMs - nowMs) / 1000);
        throw new Error(
          `Conta temporariamente bloqueada. Tente novamente em ${remainingSeconds}s.`,
        );
      }
    }

    if (user.password !== input.password) {
      user.failed_attempts += 1;
      if (user.failed_attempts >= 5) {
        const lockSecs =
          user.failed_attempts === 5
            ? 300
            : user.failed_attempts === 6
              ? 600
              : user.failed_attempts === 7
                ? 1200
                : 3600;
        user.locked_until = new Date(
          Date.now() + lockSecs * 1000,
        ).toISOString();
        saveSharedMockState(state);
        throw new Error(
          `Muitas tentativas falhas. Conta bloqueada por ${Math.floor(lockSecs / 60)} minutos.`,
        );
      }
      saveSharedMockState(state);
      throw new Error("E-mail ou senha inválidos.");
    }

    const now = new Date().toISOString();
    user.failed_attempts = 0;
    user.locked_until = null;
    user.last_login_at = now;

    const session: SessionInfo = {
      user_id: user.id,
      tenant_id: user.tenant_id,
      name: user.name,
      email: user.email,
      role_id: user.role_id,
      role_name: user.role_name,
      role_slug: user.role_slug,
      permissions: [...user.permissions],
      authenticated_at: now,
      last_activity_at: now,
      inactivity_timeout_minutes: state.config.inactivity_timeout_minutes,
    };

    state.session = session;
    saveSharedMockState(state);
    return { ...session };
  }

  async logout(): Promise<void> {
    const state = getSharedMockState();
    state.session = null;
    saveSharedMockState(state);
  }

  async getSession(): Promise<SessionInfo | null> {
    const state = getSharedMockState();
    return state.session ? { ...state.session } : null;
  }

  async recoverPasswordWithKey(input: RecoverPasswordInput): Promise<void> {
    const state = getSharedMockState();
    if (input.new_password.length < 8) {
      throw new Error("A nova senha deve ter pelo menos 8 caracteres.");
    }
    const email = input.email.trim().toLowerCase();
    const user = state.users.find((u) => u.email === email && !u.deleted_at);
    const normalizedKey = input.recovery_key.trim().toUpperCase();

    if (!user || !user.recoveryKey || user.recoveryKey !== normalizedKey) {
      throw new Error("E-mail ou chave de recuperação inválidos.");
    }

    user.password = input.new_password;
    user.failed_attempts = 0;
    user.locked_until = null;
    user.updated_at = new Date().toISOString();
    saveSharedMockState(state);
  }
}
