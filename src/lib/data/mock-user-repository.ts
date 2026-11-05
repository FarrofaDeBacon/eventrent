import {
  getSharedMockState,
  saveSharedMockState,
  type MockInternalUser,
} from "./mock-auth-repository";
import {
  PERMISSIONS,
  ROLE_SLUGS,
  type CreateUserInput,
  type ResetPasswordInput,
  type RoleWithPermissions,
  type UpdateUserInput,
  type UserWithRole,
} from "./types";
import type { UserRepository } from "./user-repository";

function toPublicUser(user: MockInternalUser): UserWithRole {
  return {
    id: user.id,
    tenant_id: user.tenant_id,
    name: user.name,
    email: user.email,
    role_id: user.role_id,
    role_name: user.role_name,
    role_slug: user.role_slug,
    permissions: [...user.permissions],
    active: user.active,
    last_login_at: user.last_login_at,
    failed_attempts: user.failed_attempts,
    locked_until: user.locked_until,
    created_at: user.created_at,
    updated_at: user.updated_at,
    deleted_at: user.deleted_at,
    created_by: user.created_by,
  };
}

function requireMockPermission(permission: string) {
  const state = getSharedMockState();
  if (!state.session) {
    throw new Error("Sessão não autenticada.");
  }
  if (!state.session.permissions.includes(permission)) {
    throw new Error(`Permissão insuficiente: requer '${permission}'.`);
  }
  return { state, session: state.session };
}

export class MockUserRepository implements UserRepository {
  async listRoles(): Promise<RoleWithPermissions[]> {
    const { state } = requireMockPermission(PERMISSIONS.ROLE_READ);
    return state.roles.map((r) => ({
      ...r,
      permissions: [...r.permissions],
    }));
  }

  async listUsers(): Promise<UserWithRole[]> {
    const { state } = requireMockPermission(PERMISSIONS.USER_READ);
    return state.users.filter((u) => !u.deleted_at).map((u) => toPublicUser(u));
  }

  async createUser(input: CreateUserInput): Promise<UserWithRole> {
    const { state, session } = requireMockPermission(PERMISSIONS.USER_MANAGE);
    if (input.name.trim().length < 2) {
      throw new Error("O nome do usuário deve ter pelo menos 2 caracteres.");
    }
    const email = input.email.trim().toLowerCase();
    if (!email.includes("@")) {
      throw new Error("Informe um e-mail válido.");
    }
    if (input.password.length < 8) {
      throw new Error("A senha deve ter pelo menos 8 caracteres.");
    }
    if (state.users.some((u) => u.email === email && !u.deleted_at)) {
      throw new Error(`Já existe um usuário ativo com o e-mail '${email}'.`);
    }
    const role = state.roles.find((r) => r.id === input.role_id);
    if (!role) {
      throw new Error("Perfil de acesso inválido.");
    }

    const now = new Date().toISOString();
    const newUser: MockInternalUser = {
      id: `user-${Date.now()}-${Math.floor(Math.random() * 1000)}`,
      tenant_id: state.defaultTenantId,
      name: input.name.trim(),
      email,
      role_id: role.id,
      role_name: role.name,
      role_slug: role.slug,
      permissions: [...role.permissions],
      active: true,
      last_login_at: null,
      failed_attempts: 0,
      locked_until: null,
      created_at: now,
      updated_at: now,
      deleted_at: null,
      created_by: session.user_id,
      password: input.password,
      recoveryKey: null,
    };

    state.users.push(newUser);
    saveSharedMockState(state);
    return toPublicUser(newUser);
  }

  async updateUser(
    userId: string,
    input: UpdateUserInput,
  ): Promise<UserWithRole> {
    const { state } = requireMockPermission(PERMISSIONS.USER_MANAGE);
    const user = state.users.find((u) => u.id === userId && !u.deleted_at);
    if (!user) {
      throw new Error("Usuário não encontrado.");
    }
    const role = state.roles.find((r) => r.id === input.role_id);
    if (!role) {
      throw new Error("Perfil de acesso inválido.");
    }

    // Regra de negócio: impedir rebaixamento do último Proprietário ativo
    if (
      user.role_slug === ROLE_SLUGS.OWNER &&
      role.slug !== ROLE_SLUGS.OWNER &&
      user.active
    ) {
      const activeOwners = state.users.filter(
        (u) => !u.deleted_at && u.active && u.role_slug === ROLE_SLUGS.OWNER,
      ).length;
      if (activeOwners <= 1) {
        throw new Error(
          "Não é permitido alterar o perfil do último Proprietário ativo.",
        );
      }
    }

    const email = input.email.trim().toLowerCase();
    if (
      state.users.some(
        (u) => u.id !== userId && u.email === email && !u.deleted_at,
      )
    ) {
      throw new Error(`Já existe outro usuário com o e-mail '${email}'.`);
    }

    user.name = input.name.trim();
    user.email = email;
    user.role_id = role.id;
    user.role_name = role.name;
    user.role_slug = role.slug;
    user.permissions = [...role.permissions];
    user.updated_at = new Date().toISOString();

    if (state.session && state.session.user_id === user.id) {
      state.session.name = user.name;
      state.session.email = user.email;
      state.session.role_id = user.role_id;
      state.session.role_name = user.role_name;
      state.session.role_slug = user.role_slug;
      state.session.permissions = [...user.permissions];
    }

    saveSharedMockState(state);
    return toPublicUser(user);
  }

  async setUserActive(userId: string, active: boolean): Promise<UserWithRole> {
    const { state, session } = requireMockPermission(PERMISSIONS.USER_MANAGE);
    if (!active && session.user_id === userId) {
      throw new Error("Você não pode desativar sua própria conta.");
    }
    const user = state.users.find((u) => u.id === userId && !u.deleted_at);
    if (!user) {
      throw new Error("Usuário não encontrado.");
    }

    if (!active && user.role_slug === ROLE_SLUGS.OWNER && user.active) {
      const activeOwners = state.users.filter(
        (u) => !u.deleted_at && u.active && u.role_slug === ROLE_SLUGS.OWNER,
      ).length;
      if (activeOwners <= 1) {
        throw new Error(
          "Não é permitido desativar o último Proprietário ativo da empresa.",
        );
      }
    }

    user.active = active;
    user.updated_at = new Date().toISOString();
    saveSharedMockState(state);
    return toPublicUser(user);
  }

  async resetUserPassword(
    userId: string,
    input: ResetPasswordInput,
  ): Promise<void> {
    const { state } = requireMockPermission(PERMISSIONS.USER_MANAGE);
    if (input.new_password.length < 8) {
      throw new Error("A nova senha deve ter pelo menos 8 caracteres.");
    }
    const user = state.users.find((u) => u.id === userId && !u.deleted_at);
    if (!user) {
      throw new Error("Usuário não encontrado.");
    }
    user.password = input.new_password;
    user.failed_attempts = 0;
    user.locked_until = null;
    user.updated_at = new Date().toISOString();
    saveSharedMockState(state);
  }
}
