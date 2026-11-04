import type { AuthRepository } from "./auth-repository";
import type { BrandingRepository } from "./branding-repository";
import { MockAuthRepository } from "./mock-auth-repository";
import { MockBrandingRepository } from "./mock-branding-repository";
import { MockUserRepository } from "./mock-user-repository";
import { TauriAuthRepository } from "./tauri-auth-repository";
import { TauriBrandingRepository } from "./tauri-branding-repository";
import { TauriUserRepository } from "./tauri-user-repository";
import { PERMISSIONS, type SessionInfo } from "./types";
import type { UserRepository } from "./user-repository";

export * from "./auth-repository";
export * from "./branding-repository";
export * from "./mock-auth-repository";
export * from "./mock-branding-repository";
export * from "./mock-user-repository";
export * from "./session.svelte";
export * from "./tauri-auth-repository";
export * from "./tauri-branding-repository";
export * from "./tauri-user-repository";
export * from "./types";
export * from "./user-repository";

let cachedBrandingRepository: BrandingRepository | null = null;
let cachedAuthRepository: AuthRepository | null = null;
let cachedUserRepository: UserRepository | null = null;

function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export function getBrandingRepository(): BrandingRepository {
  if (!cachedBrandingRepository) {
    cachedBrandingRepository = isTauriRuntime()
      ? new TauriBrandingRepository()
      : new MockBrandingRepository();
  }
  return cachedBrandingRepository;
}

export function setBrandingRepository(repo: BrandingRepository): void {
  cachedBrandingRepository = repo;
}

export function getAuthRepository(): AuthRepository {
  if (!cachedAuthRepository) {
    cachedAuthRepository = isTauriRuntime()
      ? new TauriAuthRepository()
      : new MockAuthRepository();
  }
  return cachedAuthRepository;
}

export function setAuthRepository(repo: AuthRepository): void {
  cachedAuthRepository = repo;
}

export function getUserRepository(): UserRepository {
  if (!cachedUserRepository) {
    cachedUserRepository = isTauriRuntime()
      ? new TauriUserRepository()
      : new MockUserRepository();
  }
  return cachedUserRepository;
}

export function setUserRepository(repo: UserRepository): void {
  cachedUserRepository = repo;
}

/**
 * Verifica se a sessão possui determinada permissão (`resource:action`).
 */
export function hasPermission(
  session: SessionInfo | null | undefined,
  permission: string,
): boolean {
  if (!session) return false;
  return session.permissions.includes(permission);
}

/**
 * Mapeia rotas protegidas para a permissão mínima necessária.
 */
export function canAccessRoute(
  session: SessionInfo | null | undefined,
  pathname: string,
): boolean {
  if (pathname.startsWith("/setup") || pathname.startsWith("/login")) {
    return true;
  }
  if (!session) return false;

  if (pathname.startsWith("/configuracoes/usuarios")) {
    return hasPermission(session, PERMISSIONS.USERS_VIEW);
  }
  if (pathname.startsWith("/configuracoes/perfis")) {
    return hasPermission(session, PERMISSIONS.ROLES_VIEW);
  }
  if (pathname.startsWith("/configuracoes/aparencia")) {
    return hasPermission(session, PERMISSIONS.BRANDING_VIEW);
  }

  return hasPermission(session, PERMISSIONS.DASHBOARD_VIEW);
}
