import { invoke } from "@tauri-apps/api/core";
import type { UserRepository } from "./user-repository";
import type {
  CreateUserInput,
  ResetPasswordInput,
  RoleWithPermissions,
  UpdateUserInput,
  UserWithRole,
} from "./types";

export class TauriUserRepository implements UserRepository {
  async listRoles(): Promise<RoleWithPermissions[]> {
    return await invoke<RoleWithPermissions[]>("list_roles");
  }

  async listUsers(): Promise<UserWithRole[]> {
    return await invoke<UserWithRole[]>("list_users");
  }

  async createUser(input: CreateUserInput): Promise<UserWithRole> {
    return await invoke<UserWithRole>("create_user", { input });
  }

  async updateUser(
    userId: string,
    input: UpdateUserInput,
  ): Promise<UserWithRole> {
    return await invoke<UserWithRole>("update_user", { userId, input });
  }

  async setUserActive(userId: string, active: boolean): Promise<UserWithRole> {
    return await invoke<UserWithRole>("set_user_active", { userId, active });
  }

  async resetUserPassword(
    userId: string,
    input: ResetPasswordInput,
  ): Promise<void> {
    await invoke<void>("reset_user_password", { userId, input });
  }
}
