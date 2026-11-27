import type { AuthRepository } from "./auth-repository";
import type { BackupRepository } from "./backup-repository";
import type { BrandingRepository } from "./branding-repository";
import type { CatalogRepository } from "./catalog-repository";
import type { CustomerRepository } from "./customer-repository";
import type { InventoryRepository } from "./inventory-repository";
import { MockAuthRepository } from "./mock-auth-repository";
import { MockBackupRepository } from "./mock-backup-repository";
import { MockBrandingRepository } from "./mock-branding-repository";
import { MockCatalogRepository } from "./mock-catalog-repository";
import { MockCustomerRepository } from "./mock-customer-repository";
import { MockInventoryRepository } from "./mock-inventory-repository";
import { MockSettingsRepository } from "./mock-settings-repository";
import { MockUserRepository } from "./mock-user-repository";
import { MockVenueRepository } from "./mock-venue-repository";
import type { SettingsRepository } from "./settings-repository";
import { TauriAuthRepository } from "./tauri-auth-repository";
import { TauriBackupRepository } from "./tauri-backup-repository";
import { TauriBrandingRepository } from "./tauri-branding-repository";
import { TauriCatalogRepository } from "./tauri-catalog-repository";
import { TauriCustomerRepository } from "./tauri-customer-repository";
import { TauriInventoryRepository } from "./tauri-inventory-repository";
import { TauriSettingsRepository } from "./tauri-settings-repository";
import { TauriUserRepository } from "./tauri-user-repository";
import { TauriVenueRepository } from "./tauri-venue-repository";
import { PERMISSIONS, type SessionInfo } from "./types";
import type { UserRepository } from "./user-repository";
import type { VenueRepository } from "./venue-repository";

export * from "./auth-repository";
export * from "./backup-repository";
export * from "./schemas/backup";
export * from "./branding-repository";
export * from "./catalog-repository";
export * from "./customer-repository";
export * from "./inventory-repository";
export * from "./schemas/inventory";
export * from "./session.svelte";
export * from "./settings-repository";
export * from "./tauri-auth-repository";
export * from "./tauri-backup-repository";
export * from "./tauri-branding-repository";
export * from "./tauri-catalog-repository";
export * from "./tauri-customer-repository";
export * from "./tauri-inventory-repository";
export * from "./tauri-settings-repository";
export * from "./tauri-user-repository";
export * from "./tauri-venue-repository";
export * from "./types";
export * from "./user-repository";
export * from "./venue-repository";

let cachedBrandingRepository: BrandingRepository | null = null;
let cachedAuthRepository: AuthRepository | null = null;
let cachedUserRepository: UserRepository | null = null;
let cachedCustomerRepository: CustomerRepository | null = null;
let cachedSettingsRepository: SettingsRepository | null = null;
let cachedVenueRepository: VenueRepository | null = null;
let cachedCatalogRepository: CatalogRepository | null = null;
let cachedBackupRepository: BackupRepository | null = null;
let cachedInventoryRepository: InventoryRepository | null = null;

export function isTauriRuntime(): boolean {
  return (
    typeof window !== "undefined" &&
    ("__TAURI_INTERNALS__" in window ||
      "__TAURI__" in window ||
      Boolean((window as unknown as { isTauri?: boolean }).isTauri))
  );
}

export function isDevOrMockMode(): boolean {
  return Boolean(import.meta.env.DEV) || import.meta.env.VITE_E2E === "1";
}

function requireDesktopOrDev(target: string): void {
  if (!isTauriRuntime() && !isDevOrMockMode()) {
    throw new Error(
      `[EventRent] Execute o aplicativo desktop EventRent. O repositório ${target} não pode ser instanciado em modo web fora do Tauri em produção.`,
    );
  }
}

export function getBrandingRepository(): BrandingRepository {
  requireDesktopOrDev("BrandingRepository");
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
  requireDesktopOrDev("AuthRepository");
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
  requireDesktopOrDev("UserRepository");
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

export function getCustomerRepository(): CustomerRepository {
  requireDesktopOrDev("CustomerRepository");
  if (!cachedCustomerRepository) {
    cachedCustomerRepository = isTauriRuntime()
      ? new TauriCustomerRepository()
      : new MockCustomerRepository();
  }
  return cachedCustomerRepository;
}

export function setCustomerRepository(repo: CustomerRepository): void {
  cachedCustomerRepository = repo;
}

/**
 * Verifica se a sessão possui determinada permissão (`resource:action`).
 */
export function hasPermission(
  session: SessionInfo | null | undefined,
  permission: string,
): boolean {
  if (!session) return false;
  if (session.permissions.includes(permission)) return true;
  if (permission.endsWith(":read")) {
    const impliedManage = `${permission.slice(0, -5)}:manage`;
    return session.permissions.includes(impliedManage);
  }
  return false;
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

  if (pathname.startsWith("/customers/new")) {
    return (
      hasPermission(session, PERMISSIONS.CUSTOMER_MANAGE) ||
      hasPermission(session, PERMISSIONS.SUPPLIER_MANAGE)
    );
  }
  if (pathname.startsWith("/suppliers")) {
    return hasPermission(session, PERMISSIONS.SUPPLIER_READ);
  }
  if (pathname.startsWith("/customers/")) {
    return (
      hasPermission(session, PERMISSIONS.CUSTOMER_READ) ||
      hasPermission(session, PERMISSIONS.SUPPLIER_READ)
    );
  }
  if (pathname.startsWith("/customers")) {
    return hasPermission(session, PERMISSIONS.CUSTOMER_READ);
  }
  if (pathname.startsWith("/settings/account")) {
    return true;
  }
  if (pathname.startsWith("/settings/users")) {
    return hasPermission(session, PERMISSIONS.USER_READ);
  }
  if (pathname.startsWith("/settings/roles")) {
    return hasPermission(session, PERMISSIONS.ROLE_READ);
  }
  if (pathname.startsWith("/settings/appearance")) {
    return hasPermission(session, PERMISSIONS.BRANDING_READ);
  }
  if (pathname.startsWith("/settings/operation")) {
    return hasPermission(session, PERMISSIONS.SETTINGS_READ);
  }
  if (pathname.startsWith("/venues/new")) {
    return hasPermission(session, PERMISSIONS.VENUE_MANAGE);
  }
  if (pathname.startsWith("/venues")) {
    return hasPermission(session, PERMISSIONS.VENUE_READ);
  }
  if (
    pathname.startsWith("/catalog/new") ||
    pathname.startsWith("/catalog/import-photos")
  ) {
    return hasPermission(session, PERMISSIONS.CATALOG_MANAGE);
  }
  if (pathname.startsWith("/catalog")) {
    return hasPermission(session, PERMISSIONS.CATALOG_READ);
  }
  if (pathname.startsWith("/inventory")) {
    return hasPermission(session, PERMISSIONS.INVENTORY_READ);
  }

  // Rota raiz: qualquer sessão autenticada tem acesso
  return true;
}

export function getSettingsRepository(): SettingsRepository {
  requireDesktopOrDev("SettingsRepository");
  if (!cachedSettingsRepository) {
    cachedSettingsRepository = isTauriRuntime()
      ? new TauriSettingsRepository()
      : new MockSettingsRepository();
  }
  return cachedSettingsRepository;
}

export function setSettingsRepository(repo: SettingsRepository): void {
  cachedSettingsRepository = repo;
}

export function getVenueRepository(): VenueRepository {
  requireDesktopOrDev("VenueRepository");
  if (!cachedVenueRepository) {
    cachedVenueRepository = isTauriRuntime()
      ? new TauriVenueRepository()
      : new MockVenueRepository();
  }
  return cachedVenueRepository;
}

export function setVenueRepository(repo: VenueRepository): void {
  cachedVenueRepository = repo;
}

export function getCatalogRepository(): CatalogRepository {
  requireDesktopOrDev("CatalogRepository");
  if (!cachedCatalogRepository) {
    cachedCatalogRepository = isTauriRuntime()
      ? new TauriCatalogRepository()
      : new MockCatalogRepository();
  }
  return cachedCatalogRepository;
}

export function setCatalogRepository(repo: CatalogRepository): void {
  cachedCatalogRepository = repo;
}

export function getBackupRepository(): BackupRepository {
  requireDesktopOrDev("BackupRepository");
  if (!cachedBackupRepository) {
    cachedBackupRepository = isTauriRuntime()
      ? new TauriBackupRepository()
      : new MockBackupRepository();
  }
  return cachedBackupRepository;
}

export function setBackupRepository(repo: BackupRepository): void {
  cachedBackupRepository = repo;
}

export function getInventoryRepository(): InventoryRepository {
  requireDesktopOrDev("InventoryRepository");
  if (!cachedInventoryRepository) {
    cachedInventoryRepository = isTauriRuntime()
      ? new TauriInventoryRepository()
      : new MockInventoryRepository();
  }
  return cachedInventoryRepository;
}

export function setInventoryRepository(repo: InventoryRepository): void {
  cachedInventoryRepository = repo;
}
