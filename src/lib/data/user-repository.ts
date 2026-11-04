import type {
  CreateUserInput,
  ResetPasswordInput,
  RoleWithPermissions,
  UpdateUserInput,
  UserWithRole,
} from "./types";

/**
 * Contrato da camada de dados para gestão de usuários e consulta de perfis de permissão.
 */
export interface UserRepository {
  listRoles(): Promise<RoleWithPermissions[]>;
  listUsers(): Promise<UserWithRole[]>;
  createUser(input: CreateUserInput): Promise<UserWithRole>;
  updateUser(userId: string, input: UpdateUserInput): Promise<UserWithRole>;
  setUserActive(userId: string, active: boolean): Promise<UserWithRole>;
  resetUserPassword(userId: string, input: ResetPasswordInput): Promise<void>;
}
