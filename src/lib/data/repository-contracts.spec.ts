import { describe, it, expect, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { render } from "@testing-library/svelte";
import RecoveryKeyModal from "$lib/components/recovery-key-modal.svelte";

import { TauriAuthRepository } from "./tauri-auth-repository";
import { TauriUserRepository } from "./tauri-user-repository";
import { TauriBrandingRepository } from "./tauri-branding-repository";
import { TauriCustomerRepository } from "./tauri-customer-repository";
import { TauriVenueRepository } from "./tauri-venue-repository";
import { TauriCatalogRepository } from "./tauri-catalog-repository";
import { TauriSettingsRepository } from "./tauri-settings-repository";
import { TauriBackupRepository } from "./tauri-backup-repository";

import { commandReturnSchemas } from "./schemas";

interface ContractFixture {
  command: string;
  is_public: boolean;
  has_return_value: boolean;
  sample_output: unknown;
  sample_errors: Record<string, string>;
}

const fixtureModules = import.meta.glob<ContractFixture>(
  "./fixtures/contracts/*.json",
  { eager: true, import: "default" },
);

function getFixture(commandName: string): ContractFixture {
  const fixture = fixtureModules[`./fixtures/contracts/${commandName}.json`];
  if (!fixture) {
    throw new Error(`Fixture canônica não encontrada para '${commandName}'`);
  }
  return fixture;
}

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("R3 / Tarefa 0.3 — Contratos Reais Rust ↔ TypeScript e Cobertura dos 62 Comandos IPC", () => {
  const mockedInvoke = vi.mocked(invoke);
  const coveredCommands = new Set<string>();

  beforeEach(() => {
    vi.resetAllMocks();
  });

  describe("1. Repositório de Autenticação (TauriAuthRepository)", () => {
    const repo = new TauriAuthRepository();

    it("get_app_status: retorna dados válidos conformes ao schema e tipo", async () => {
      coveredCommands.add("get_app_status");
      const fixture = getFixture("get_app_status");
      mockedInvoke.mockResolvedValueOnce(fixture.sample_output);

      const result = await repo.getAppStatus();
      expect(result).toBeDefined();
      expect(
        commandReturnSchemas["get_app_status"].safeParse(result).success,
      ).toBe(true);
      expect(typeof result.has_owner).toBe("boolean");
      expect(mockedInvoke).toHaveBeenCalledWith("get_app_status");
    });

    it("complete_initial_setup: retorna setup completo e schema válido", async () => {
      coveredCommands.add("complete_initial_setup");
      const fixture = getFixture("complete_initial_setup");
      mockedInvoke.mockResolvedValueOnce(fixture.sample_output);

      const result = await repo.completeInitialSetup({
        company_name: "Empresa",
        owner_name: "Owner",
        owner_email: "owner@test.com",
        owner_password: "Password123!",
        mode: "local",
      });
      expect(result).toBeDefined();
      expect(
        commandReturnSchemas["complete_initial_setup"].safeParse(result)
          .success,
      ).toBe(true);
      expect(result.recovery_key).toBeDefined();
      expect(mockedInvoke).toHaveBeenCalledWith(
        "complete_initial_setup",
        expect.anything(),
      );
    });

    it("login: retorna sessão válida", async () => {
      coveredCommands.add("login");
      const fixture = getFixture("login");
      mockedInvoke.mockResolvedValueOnce(fixture.sample_output);

      const result = await repo.login({
        email: "user@test.com",
        password: "pwd",
      });
      expect(result).toBeDefined();
      expect(commandReturnSchemas["login"].safeParse(result).success).toBe(
        true,
      );
      expect(result.user_id).toBeDefined();
      expect(mockedInvoke).toHaveBeenCalledWith("login", expect.anything());
    });

    it("logout: comando void", async () => {
      coveredCommands.add("logout");
      mockedInvoke.mockResolvedValueOnce(undefined);
      const res = await repo.logout();
      expect(res).toBeUndefined();
      expect(mockedInvoke).toHaveBeenCalledWith("logout");
    });

    it("get_current_session: retorna sessão ou null", async () => {
      coveredCommands.add("get_current_session");
      const fixture = getFixture("get_current_session");
      mockedInvoke.mockResolvedValueOnce(fixture.sample_output);

      const result = await repo.getSession();
      expect(result).toBeDefined();
      expect(
        commandReturnSchemas["get_current_session"].safeParse(result).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith("get_current_session");
    });

    it("recover_password: NÃO retorna void; devolve { new_recovery_key } válida", async () => {
      coveredCommands.add("recover_password");
      const fixture = getFixture("recover_password");
      mockedInvoke.mockResolvedValueOnce(fixture.sample_output);

      const result = await repo.recoverPasswordWithKey({
        email: "carlos@test.com",
        recovery_key: "ER-KEY-123",
        new_password: "NovaSenha123!",
      });

      expect(result).toBeDefined();
      expect(result).not.toBeNull();
      expect(typeof result.new_recovery_key).toBe("string");
      expect(
        commandReturnSchemas["recover_password"].safeParse(result).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith(
        "recover_password",
        expect.anything(),
      );
    });

    it("regenerate_recovery_key: devolve nova chave conforme fixture", async () => {
      coveredCommands.add("regenerate_recovery_key");
      const fixture = getFixture("regenerate_recovery_key");
      mockedInvoke.mockResolvedValueOnce(fixture.sample_output);

      const result = await repo.regenerateRecoveryKey({
        current_password: "SenhaAtual123!",
      });

      expect(result).toBeDefined();
      expect(typeof result.new_recovery_key).toBe("string");
      expect(
        commandReturnSchemas["regenerate_recovery_key"].safeParse(result)
          .success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith(
        "regenerate_recovery_key",
        expect.anything(),
      );
    });

    it("Erros de negócio em login chegam intactos à interface", async () => {
      const fixture = getFixture("login");
      const errorMsg = fixture.sample_errors["AccountLocked"];
      mockedInvoke.mockRejectedValueOnce(new Error(errorMsg));

      await expect(
        repo.login({ email: "test@test.com", password: "pwd" }),
      ).rejects.toThrow(errorMsg);
    });
  });

  describe("2. Repositório de Usuários (TauriUserRepository)", () => {
    const repo = new TauriUserRepository();

    it("list_roles: retorna lista de perfis", async () => {
      coveredCommands.add("list_roles");
      const rolesFix = getFixture("list_roles");
      mockedInvoke.mockResolvedValueOnce(rolesFix.sample_output);
      const roles = await repo.listRoles();
      expect(commandReturnSchemas["list_roles"].safeParse(roles).success).toBe(
        true,
      );
      expect(mockedInvoke).toHaveBeenCalledWith("list_roles");
    });

    it("list_users: retorna lista de usuários", async () => {
      coveredCommands.add("list_users");
      const usersFix = getFixture("list_users");
      mockedInvoke.mockResolvedValueOnce(usersFix.sample_output);
      const users = await repo.listUsers();
      expect(commandReturnSchemas["list_users"].safeParse(users).success).toBe(
        true,
      );
      expect(mockedInvoke).toHaveBeenCalledWith("list_users");
    });

    it("create_user: cria usuário e retorna UserWithRole", async () => {
      coveredCommands.add("create_user");
      const createFix = getFixture("create_user");
      mockedInvoke.mockResolvedValueOnce(createFix.sample_output);
      const created = await repo.createUser({
        name: "Test",
        email: "test@test.com",
        password: "pwd",
        role_id: "role-1",
      });
      expect(
        commandReturnSchemas["create_user"].safeParse(created).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith(
        "create_user",
        expect.anything(),
      );
    });

    it("update_user: atualiza usuário e retorna UserWithRole", async () => {
      coveredCommands.add("update_user");
      const updateFix = getFixture("update_user");
      mockedInvoke.mockResolvedValueOnce(updateFix.sample_output);
      const updated = await repo.updateUser("u-1", {
        name: "Test 2",
        email: "test2@test.com",
        role_id: "role-1",
      });
      expect(
        commandReturnSchemas["update_user"].safeParse(updated).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith(
        "update_user",
        expect.anything(),
      );
    });

    it("set_user_active: altera status de usuário", async () => {
      coveredCommands.add("set_user_active");
      const fix = getFixture("set_user_active");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const res = await repo.setUserActive("u-1", false);
      expect(
        commandReturnSchemas["set_user_active"].safeParse(res).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith(
        "set_user_active",
        expect.anything(),
      );
    });

    it("reset_user_password: void", async () => {
      coveredCommands.add("reset_user_password");
      mockedInvoke.mockResolvedValueOnce(undefined);
      const res = await repo.resetUserPassword("u-1", {
        new_password: "NewPassword123!",
      });
      expect(res).toBeUndefined();
      expect(mockedInvoke).toHaveBeenCalledWith(
        "reset_user_password",
        expect.anything(),
      );
    });
  });

  describe("3. Repositório de Branding (TauriBrandingRepository)", () => {
    const brandingRepo = new TauriBrandingRepository();

    it("get_public_branding: retorna branding público ou null", async () => {
      coveredCommands.add("get_public_branding");
      const pubFix = getFixture("get_public_branding");
      mockedInvoke.mockResolvedValueOnce(pubFix.sample_output);
      const pub = await brandingRepo.getPublicBranding();
      expect(
        commandReturnSchemas["get_public_branding"].safeParse(pub).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith("get_public_branding");
    });

    it("get_branding: retorna branding tenant", async () => {
      coveredCommands.add("get_branding");
      const fix = getFixture("get_branding");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const branding = await brandingRepo.getBranding("tenant-1");
      expect(
        commandReturnSchemas["get_branding"].safeParse(branding).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith(
        "get_branding",
        expect.anything(),
      );
    });

    it("save_branding: salva e retorna TenantBranding", async () => {
      coveredCommands.add("save_branding");
      const saveFix = getFixture("save_branding");
      mockedInvoke.mockResolvedValueOnce(saveFix.sample_output);
      const saved = await brandingRepo.saveBranding("tenant-1", {
        company_name: "Empresa",
        primary_color: "#112233",
        secondary_color: "#445566",
        accent_color: "#778899",
      });
      expect(
        commandReturnSchemas["save_branding"].safeParse(saved).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith(
        "save_branding",
        expect.anything(),
      );
    });
  });

  describe("4. Repositório de Clientes (TauriCustomerRepository)", () => {
    const repo = new TauriCustomerRepository();

    it("list_customers: valida página de clientes", async () => {
      coveredCommands.add("list_customers");
      const fix = getFixture("list_customers");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const page = await repo.listCustomers({});
      expect(
        commandReturnSchemas["list_customers"].safeParse(page).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith(
        "list_customers",
        expect.anything(),
      );
    });

    it("get_customer: valida detalhes do cliente", async () => {
      coveredCommands.add("get_customer");
      const fix = getFixture("get_customer");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const details = await repo.getCustomer("cust-1");
      expect(
        commandReturnSchemas["get_customer"].safeParse(details).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith(
        "get_customer",
        expect.anything(),
      );
    });

    it("create_customer: cria e valida detalhes", async () => {
      coveredCommands.add("create_customer");
      const fix = getFixture("create_customer");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const created = await repo.createCustomer({
        person_type: "legal",
        name: "Empresa",
        document_type: "cnpj",
        document_number: "000",
        is_customer: true,
        is_supplier: false,
        contacts: [],
        addresses: [],
      });
      expect(
        commandReturnSchemas["create_customer"].safeParse(created).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith(
        "create_customer",
        expect.anything(),
      );
    });

    it("update_customer: atualiza e valida detalhes", async () => {
      coveredCommands.add("update_customer");
      const fix = getFixture("update_customer");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const updated = await repo.updateCustomer("cust-1", {
        person_type: "legal",
        name: "Empresa Atualizada",
        document_type: "cnpj",
        document_number: "000",
        is_customer: true,
        is_supplier: false,
        contacts: [],
        addresses: [],
        expected_updated_at: "2026-01-01T00:00:00Z",
      });
      expect(
        commandReturnSchemas["update_customer"].safeParse(updated).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith(
        "update_customer",
        expect.anything(),
      );
    });

    it("set_customer_active: ativa/desativa cliente", async () => {
      coveredCommands.add("set_customer_active");
      const fix = getFixture("set_customer_active");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const res = await repo.setCustomerActive("cust-1", false);
      expect(
        commandReturnSchemas["set_customer_active"].safeParse(res).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith(
        "set_customer_active",
        expect.anything(),
      );
    });

    it("delete_customer: void", async () => {
      coveredCommands.add("delete_customer");
      mockedInvoke.mockResolvedValueOnce(undefined);
      const res = await repo.deleteCustomer("cust-1");
      expect(res).toBeUndefined();
      expect(mockedInvoke).toHaveBeenCalledWith(
        "delete_customer",
        expect.anything(),
      );
    });

    it("add_customer_role: adiciona papel ao cliente", async () => {
      coveredCommands.add("add_customer_role");
      const fix = getFixture("add_customer_role");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const res = await repo.addCustomerRole("cust-1", "supplier");
      expect(
        commandReturnSchemas["add_customer_role"].safeParse(res).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith(
        "add_customer_role",
        expect.anything(),
      );
    });

    it("Erro de validação (CNPJ inválido) propaga mensagem intacta", async () => {
      const fix = getFixture("create_customer");
      const errorMsg = fix.sample_errors["Validation"];
      mockedInvoke.mockRejectedValueOnce(new Error(errorMsg));

      await expect(
        repo.createCustomer({
          person_type: "legal",
          name: "Empresa",
          document_type: "cnpj",
          document_number: "000",
          is_customer: true,
          is_supplier: false,
          contacts: [],
          addresses: [],
        }),
      ).rejects.toThrow(errorMsg);
    });
  });

  describe("5. Repositório de Locais (TauriVenueRepository)", () => {
    const repo = new TauriVenueRepository();

    it("list_venues: valida lista paginada de locais", async () => {
      coveredCommands.add("list_venues");
      const listFix = getFixture("list_venues");
      mockedInvoke.mockResolvedValueOnce(listFix.sample_output);
      const list = await repo.list({});
      expect(commandReturnSchemas["list_venues"].safeParse(list).success).toBe(
        true,
      );
      expect(mockedInvoke).toHaveBeenCalledWith(
        "list_venues",
        expect.anything(),
      );
    });

    it("get_venue: valida busca de local por id", async () => {
      coveredCommands.add("get_venue");
      const getFix = getFixture("get_venue");
      mockedInvoke.mockResolvedValueOnce(getFix.sample_output);
      const venue = await repo.getById("venue-1");
      expect(commandReturnSchemas["get_venue"].safeParse(venue).success).toBe(
        true,
      );
      expect(mockedInvoke).toHaveBeenCalledWith("get_venue", expect.anything());
    });

    it("create_venue: cria e valida local", async () => {
      coveredCommands.add("create_venue");
      const fix = getFixture("create_venue");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const venue = await repo.create({
        name: "Teatro Municipal",
        city: "São Paulo",
        state: "SP",
        country: "BR",
        spaces: [],
        contacts: [],
      });
      expect(
        commandReturnSchemas["create_venue"].safeParse(venue).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith(
        "create_venue",
        expect.anything(),
      );
    });

    it("update_venue: atualiza local", async () => {
      coveredCommands.add("update_venue");
      const fix = getFixture("update_venue");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const venue = await repo.update({
        id: "venue-1",
        name: "Teatro Municipal Reformado",
        city: "São Paulo",
        state: "SP",
        country: "BR",
        spaces: [],
        contacts: [],
        expected_updated_at: "2026-01-01T00:00:00Z",
      });
      expect(
        commandReturnSchemas["update_venue"].safeParse(venue).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith(
        "update_venue",
        expect.anything(),
      );
    });

    it("set_venue_active: altera status de local", async () => {
      coveredCommands.add("set_venue_active");
      const fix = getFixture("set_venue_active");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const venue = await repo.setActive("venue-1", false);
      expect(
        commandReturnSchemas["set_venue_active"].safeParse(venue).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith(
        "set_venue_active",
        expect.anything(),
      );
    });

    it("delete_venue: void", async () => {
      coveredCommands.add("delete_venue");
      mockedInvoke.mockResolvedValueOnce(undefined);
      const res = await repo.delete("venue-1");
      expect(res).toBeUndefined();
      expect(mockedInvoke).toHaveBeenCalledWith(
        "delete_venue",
        expect.anything(),
      );
    });

    it("check_venue_duplicate_name_city: retorna boolean", async () => {
      coveredCommands.add("check_venue_duplicate_name_city");
      const fix = getFixture("check_venue_duplicate_name_city");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const duplicate = await repo.checkDuplicateNameCity(
        "Teatro",
        "São Paulo",
      );
      expect(typeof duplicate).toBe("boolean");
      expect(mockedInvoke).toHaveBeenCalledWith(
        "check_venue_duplicate_name_city",
        expect.anything(),
      );
    });
  });

  describe("6. Repositório de Configurações (TauriSettingsRepository)", () => {
    const settingsRepo = new TauriSettingsRepository();

    it("get_allocation_settings: valida schema", async () => {
      coveredCommands.add("get_allocation_settings");
      const getFix = getFixture("get_allocation_settings");
      mockedInvoke.mockResolvedValueOnce(getFix.sample_output);
      const settings = await settingsRepo.getAllocationSettings();
      expect(
        commandReturnSchemas["get_allocation_settings"].safeParse(settings)
          .success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith("get_allocation_settings");
    });

    it("update_allocation_settings: valida atualização de configurações", async () => {
      coveredCommands.add("update_allocation_settings");
      const fix = getFixture("update_allocation_settings");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const res = await settingsRepo.updateAllocationSettings({
        changeover_minutes: 60,
        internal_transfer_minutes: 30,
        external_transfer_minutes: 45,
      });
      expect(
        commandReturnSchemas["update_allocation_settings"].safeParse(res)
          .success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith(
        "update_allocation_settings",
        expect.anything(),
      );
    });
  });

  describe("7. Repositório de Catálogo - Marcas, Categorias e Atributos", () => {
    const repo = new TauriCatalogRepository();

    it("list_brands: lista marcas", async () => {
      coveredCommands.add("list_brands");
      const fix = getFixture("list_brands");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const brands = await repo.listBrands();
      expect(
        commandReturnSchemas["list_brands"].safeParse(brands).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith("list_brands", {
        includeInactive: false,
      });
    });

    it("create_brand: cria marca", async () => {
      coveredCommands.add("create_brand");
      const fix = getFixture("create_brand");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const brand = await repo.createBrand({ name: "Pioneer", active: true });
      expect(
        commandReturnSchemas["create_brand"].safeParse(brand).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith(
        "create_brand",
        expect.anything(),
      );
    });

    it("update_brand: atualiza marca", async () => {
      coveredCommands.add("update_brand");
      const fix = getFixture("update_brand");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const brand = await repo.updateBrand("brand-1", {
        name: "Pioneer DJ",
        active: true,
        expected_updated_at: "2026-01-01T00:00:00Z",
      });
      expect(
        commandReturnSchemas["update_brand"].safeParse(brand).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith(
        "update_brand",
        expect.anything(),
      );
    });

    it("delete_brand: exclui marca", async () => {
      coveredCommands.add("delete_brand");
      mockedInvoke.mockResolvedValueOnce(undefined);
      const res = await repo.deleteBrand("brand-1");
      expect(res).toBeUndefined();
      expect(mockedInvoke).toHaveBeenCalledWith("delete_brand", {
        id: "brand-1",
      });
    });

    it("list_categories_tree: lista árvore hierárquica", async () => {
      coveredCommands.add("list_categories_tree");
      const fix = getFixture("list_categories_tree");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const tree = await repo.listCategoriesTree();
      expect(
        commandReturnSchemas["list_categories_tree"].safeParse(tree).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith("list_categories_tree", {
        includeInactive: false,
      });
    });

    it("get_category_effective_attributes: busca atributos efetivos", async () => {
      coveredCommands.add("get_category_effective_attributes");
      const fix = getFixture("get_category_effective_attributes");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const attrs = await repo.getCategoryEffectiveAttributes("cat-1");
      expect(
        commandReturnSchemas["get_category_effective_attributes"].safeParse(
          attrs,
        ).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith(
        "get_category_effective_attributes",
        { categoryId: "cat-1" },
      );
    });

    it("create_category: cria categoria", async () => {
      coveredCommands.add("create_category");
      const fix = getFixture("create_category");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const cat = await repo.createCategory({
        parent_id: null,
        name: "Sonorização",
        description: null,
        sort_order: 1,
        active: true,
      });
      expect(
        commandReturnSchemas["create_category"].safeParse(cat).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith(
        "create_category",
        expect.anything(),
      );
    });

    it("update_category: atualiza categoria", async () => {
      coveredCommands.add("update_category");
      const fix = getFixture("update_category");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const cat = await repo.updateCategory("cat-1", {
        parent_id: null,
        name: "Áudio e Som",
        description: null,
        sort_order: 1,
        active: true,
        expected_updated_at: "2026-01-01T00:00:00Z",
      });
      expect(
        commandReturnSchemas["update_category"].safeParse(cat).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith(
        "update_category",
        expect.anything(),
      );
    });

    it("delete_category: exclui categoria", async () => {
      coveredCommands.add("delete_category");
      mockedInvoke.mockResolvedValueOnce(undefined);
      const res = await repo.deleteCategory("cat-1");
      expect(res).toBeUndefined();
      expect(mockedInvoke).toHaveBeenCalledWith("delete_category", {
        id: "cat-1",
      });
    });

    it("create_attribute_definition: cria atributo", async () => {
      coveredCommands.add("create_attribute_definition");
      const fix = getFixture("create_attribute_definition");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const attr = await repo.createAttributeDefinition({
        category_id: "cat-1",
        field_key: "potencia",
        label: "Potência",
        data_type: "number",
        unit: "W",
        choices: [],
        is_required: false,
        sort_order: 1,
        active: true,
      });
      expect(
        commandReturnSchemas["create_attribute_definition"].safeParse(attr)
          .success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith(
        "create_attribute_definition",
        expect.anything(),
      );
    });

    it("update_attribute_definition: atualiza atributo", async () => {
      coveredCommands.add("update_attribute_definition");
      const fix = getFixture("update_attribute_definition");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const attr = await repo.updateAttributeDefinition("attr-1", {
        field_key: "potencia",
        label: "Potência RMS",
        data_type: "number",
        unit: "W",
        choices: [],
        is_required: true,
        sort_order: 1,
        active: true,
        expected_updated_at: "2026-01-01T00:00:00Z",
      });
      expect(
        commandReturnSchemas["update_attribute_definition"].safeParse(attr)
          .success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith(
        "update_attribute_definition",
        expect.anything(),
      );
    });

    it("delete_attribute_definition: exclui atributo", async () => {
      coveredCommands.add("delete_attribute_definition");
      mockedInvoke.mockResolvedValueOnce(undefined);
      const res = await repo.deleteAttributeDefinition("attr-1");
      expect(res).toBeUndefined();
      expect(mockedInvoke).toHaveBeenCalledWith("delete_attribute_definition", {
        id: "attr-1",
      });
    });
  });

  describe("8. Repositório de Catálogo - Modelos de Equipamento", () => {
    const repo = new TauriCatalogRepository();

    it("load_suggested_catalog_categories: carrega categorias sugeridas", async () => {
      coveredCommands.add("load_suggested_catalog_categories");
      const fix = getFixture("load_suggested_catalog_categories");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const res = await repo.loadSuggestedCategories();
      expect(
        commandReturnSchemas["load_suggested_catalog_categories"].safeParse(res)
          .success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith(
        "load_suggested_catalog_categories",
      );
    });

    it("list_equipment_models: lista paginada de modelos", async () => {
      coveredCommands.add("list_equipment_models");
      const fix = getFixture("list_equipment_models");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const models = await repo.listEquipmentModels({ limit: 20 });
      expect(
        commandReturnSchemas["list_equipment_models"].safeParse(models).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith(
        "list_equipment_models",
        expect.anything(),
      );
    });

    it("get_equipment_model: busca modelo por ID", async () => {
      coveredCommands.add("get_equipment_model");
      const fix = getFixture("get_equipment_model");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const model = await repo.getEquipmentModel("mod-1");
      expect(
        commandReturnSchemas["get_equipment_model"].safeParse(model).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith("get_equipment_model", {
        id: "mod-1",
      });
    });

    it("create_equipment_model: cria modelo", async () => {
      coveredCommands.add("create_equipment_model");
      const fix = getFixture("create_equipment_model");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const model = await repo.createEquipmentModel({
        category_id: "cat-1",
        brand_id: null,
        name: "CDJ-3000",
        model_code: null,
        sku: null,
        ean: null,
        ncm: null,
        description: null,
        control_mode: "serialized",
        weight_kg: null,
        length_cm: null,
        width_cm: null,
        height_cm: null,
        power_watts: null,
        voltage: null,
        amperage: null,
        requires_case: false,
        units_per_case: null,
        daily_rate_cents: 15000,
        replacement_value_cents: null,
        cost_price_cents: null,
        active: true,
        attribute_values: [],
      });
      expect(
        commandReturnSchemas["create_equipment_model"].safeParse(model).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith(
        "create_equipment_model",
        expect.anything(),
      );
    });

    it("update_equipment_model: atualiza modelo", async () => {
      coveredCommands.add("update_equipment_model");
      const fix = getFixture("update_equipment_model");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const model = await repo.updateEquipmentModel("mod-1", {
        category_id: "cat-1",
        brand_id: null,
        name: "CDJ-3000 Plus",
        model_code: null,
        sku: null,
        ean: null,
        ncm: null,
        description: null,
        control_mode: "serialized",
        weight_kg: null,
        length_cm: null,
        width_cm: null,
        height_cm: null,
        power_watts: null,
        voltage: null,
        amperage: null,
        requires_case: false,
        units_per_case: null,
        daily_rate_cents: 18000,
        replacement_value_cents: null,
        cost_price_cents: null,
        active: true,
        attribute_values: [],
        expected_updated_at: "2026-01-01T00:00:00Z",
      });
      expect(
        commandReturnSchemas["update_equipment_model"].safeParse(model).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith(
        "update_equipment_model",
        expect.anything(),
      );
    });

    it("set_equipment_model_active: ativa/desativa modelo", async () => {
      coveredCommands.add("set_equipment_model_active");
      mockedInvoke.mockResolvedValueOnce(undefined);
      const res = await repo.setEquipmentModelActive("mod-1", false);
      expect(res).toBeUndefined();
      expect(mockedInvoke).toHaveBeenCalledWith("set_equipment_model_active", {
        id: "mod-1",
        active: false,
      });
    });

    it("delete_equipment_model: exclui modelo", async () => {
      coveredCommands.add("delete_equipment_model");
      mockedInvoke.mockResolvedValueOnce(undefined);
      const res = await repo.deleteEquipmentModel("mod-1");
      expect(res).toBeUndefined();
      expect(mockedInvoke).toHaveBeenCalledWith("delete_equipment_model", {
        id: "mod-1",
      });
    });
  });

  describe("9. Repositório de Catálogo - Mídia e Blobs Binários (11 Comandos)", () => {
    const repo = new TauriCatalogRepository();

    it("list_model_media: lista fotos e documentos do modelo", async () => {
      coveredCommands.add("list_model_media");
      const fix = getFixture("list_model_media");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const mediaList = await repo.listModelMedia("mod-1");
      expect(
        commandReturnSchemas["list_model_media"].safeParse(mediaList).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith("list_model_media", {
        modelId: "mod-1",
      });
    });

    it("get_media_blob_bytes: retorna bytes binários Uint8Array via canal de resposta", async () => {
      coveredCommands.add("get_media_blob_bytes");
      const fix = getFixture("get_media_blob_bytes");
      mockedInvoke.mockResolvedValueOnce(
        new Uint8Array(fix.sample_output as number[]),
      );
      const bytes = await repo.getMediaBlobBytes("blob-1");
      expect(bytes).toBeInstanceOf(Uint8Array);
      expect(bytes.byteLength).toBe(4);
      expect(
        commandReturnSchemas["get_media_blob_bytes"].safeParse(bytes).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith("get_media_blob_bytes", {
        blobId: "blob-1",
      });
    });

    it("attach_model_photo: empacota frame binário raw e anexa foto ao modelo", async () => {
      coveredCommands.add("attach_model_photo");
      const fix = getFixture("attach_model_photo");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const res = await repo.attachModelPhoto({
        model_id: "mod-1",
        title: "Foto Frontal",
        origin: "manufacturer",
        credit_note: null,
        original_filename: "foto.png",
        display_mime_type: "image/webp",
        display_bytes: new Uint8Array([1, 2, 3]),
        thumb_mime_type: "image/webp",
        thumb_bytes: new Uint8Array([4, 5]),
      });
      expect(
        commandReturnSchemas["attach_model_photo"].safeParse(res).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith(
        "attach_model_photo",
        expect.any(Uint8Array),
      );
    });

    it("attach_model_document: empacota frame binário raw e anexa PDF ao modelo", async () => {
      coveredCommands.add("attach_model_document");
      const fix = getFixture("attach_model_document");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const res = await repo.attachModelDocument({
        model_id: "mod-1",
        title: "Manual Técnico",
        doc_type: "manual",
        origin: "manufacturer",
        credit_note: null,
        language: "pt-BR",
        version_note: "1.0",
        original_filename: "manual.pdf",
        mime_type: "application/pdf",
        bytes: new Uint8Array([37, 80, 68, 70]), // %PDF
      });
      expect(
        commandReturnSchemas["attach_model_document"].safeParse(res).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith(
        "attach_model_document",
        expect.any(Uint8Array),
      );
    });

    it("update_model_media_metadata: atualiza título e metadados", async () => {
      coveredCommands.add("update_model_media_metadata");
      const fix = getFixture("update_model_media_metadata");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const res = await repo.updateModelMediaMetadata({
        media_link_id: "link-1",
        title: "Manual Atualizado",
        doc_type: "manual",
        origin: "manufacturer",
        credit_note: null,
        language: "pt-BR",
        version_note: "1.1",
        expected_updated_at: "2026-01-01T00:00:00Z",
      });
      expect(
        commandReturnSchemas["update_model_media_metadata"].safeParse(res)
          .success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith(
        "update_model_media_metadata",
        expect.anything(),
      );
    });

    it("reorder_model_media: reordena mídias", async () => {
      coveredCommands.add("reorder_model_media");
      const fix = getFixture("reorder_model_media");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const res = await repo.reorderModelMedia({
        model_id: "mod-1",
        kind: "image",
        ordered_link_ids: ["link-2", "link-1"],
      });
      expect(
        commandReturnSchemas["reorder_model_media"].safeParse(res).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith(
        "reorder_model_media",
        expect.anything(),
      );
    });

    it("set_primary_model_photo: define foto principal", async () => {
      coveredCommands.add("set_primary_model_photo");
      const fix = getFixture("set_primary_model_photo");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const res = await repo.setPrimaryModelPhoto("link-1");
      expect(
        commandReturnSchemas["set_primary_model_photo"].safeParse(res).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith("set_primary_model_photo", {
        mediaLinkId: "link-1",
      });
    });

    it("remove_model_media: remove vínculo de mídia", async () => {
      coveredCommands.add("remove_model_media");
      mockedInvoke.mockResolvedValueOnce(undefined);
      const res = await repo.removeModelMedia("link-1");
      expect(res).toBeUndefined();
      expect(mockedInvoke).toHaveBeenCalledWith("remove_model_media", {
        mediaLinkId: "link-1",
      });
    });

    it("collect_unreferenced_media: executa coleta de lixo", async () => {
      coveredCommands.add("collect_unreferenced_media");
      const fix = getFixture("collect_unreferenced_media");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const gcResult = await repo.collectUnreferencedMedia();
      expect(
        commandReturnSchemas["collect_unreferenced_media"].safeParse(gcResult)
          .success,
      ).toBe(true);
      expect(typeof gcResult.deleted_files).toBe("number");
      expect(mockedInvoke).toHaveBeenCalledWith("collect_unreferenced_media");
    });

    it("open_model_document_external: abre PDF no leitor padrão", async () => {
      coveredCommands.add("open_model_document_external");
      const fix = getFixture("open_model_document_external");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const path = await repo.openModelDocumentExternal("link-1");
      expect(typeof path).toBe("string");
      expect(
        commandReturnSchemas["open_model_document_external"].safeParse(path)
          .success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith(
        "open_model_document_external",
        { mediaLinkId: "link-1" },
      );
    });

    it("save_model_document_as: exporta cópia sanitizada de PDF", async () => {
      coveredCommands.add("save_model_document_as");
      const fix = getFixture("save_model_document_as");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const path = await repo.saveModelDocumentAs("link-1");
      expect(
        commandReturnSchemas["save_model_document_as"].safeParse(path).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith("save_model_document_as", {
        mediaLinkId: "link-1",
      });
    });
  });

  describe("8. Repositório de Backup e Restauração (TauriBackupRepository)", () => {
    const repo = new TauriBackupRepository();

    it("get_backup_status: consulta status e métricas do backup", async () => {
      coveredCommands.add("get_backup_status");
      const fix = getFixture("get_backup_status");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const res = await repo.getStatus();
      expect(
        commandReturnSchemas["get_backup_status"].safeParse(res).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith("get_backup_status");
    });

    it("set_backup_settings: atualiza configurações de retenção e agendamento", async () => {
      coveredCommands.add("set_backup_settings");
      const fix = getFixture("set_backup_settings");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const input = {
        destination_folder: "C:\\Backups\\EventRent",
        schedule_enabled: true,
        retention_daily: 7,
        retention_weekly: 4,
        retention_monthly: 6,
      };
      const res = await repo.updateSettings(input);
      expect(
        commandReturnSchemas["set_backup_settings"].safeParse(res).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith("set_backup_settings", {
        input,
      });
    });

    it("run_backup_now: executa backup imediato consistente", async () => {
      coveredCommands.add("run_backup_now");
      const fix = getFixture("run_backup_now");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const res = await repo.runBackupNow();
      expect(
        commandReturnSchemas["run_backup_now"].safeParse(res).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith("run_backup_now");
    });

    it("list_backups: lista backups na pasta configurada", async () => {
      coveredCommands.add("list_backups");
      const fix = getFixture("list_backups");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const res = await repo.listBackups();
      expect(commandReturnSchemas["list_backups"].safeParse(res).success).toBe(
        true,
      );
      expect(mockedInvoke).toHaveBeenCalledWith("list_backups", {
        input: { folder: null },
      });
    });

    it("verify_backup: valida integridade física SHA-256 e compatibilidade de migrações", async () => {
      coveredCommands.add("verify_backup");
      const fix = getFixture("verify_backup");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const res = await repo.verifyBackup(
        "C:\\Backups\\EventRent\\backup.eventrent-backup",
      );
      expect(commandReturnSchemas["verify_backup"].safeParse(res).success).toBe(
        true,
      );
      expect(mockedInvoke).toHaveBeenCalledWith("verify_backup", {
        input: { file_path: "C:\\Backups\\EventRent\\backup.eventrent-backup" },
      });
    });

    it("schedule_restore: agenda restauração a frio para próximo boot", async () => {
      coveredCommands.add("schedule_restore");
      const fix = getFixture("schedule_restore");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const res = await repo.scheduleRestore(
        "C:\\Backups\\EventRent\\backup.eventrent-backup",
      );
      expect(
        commandReturnSchemas["schedule_restore"].safeParse(res).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith("schedule_restore", {
        input: { file_path: "C:\\Backups\\EventRent\\backup.eventrent-backup" },
      });
    });

    it("choose_backup_folder: abre seletor nativo de pasta de backup", async () => {
      coveredCommands.add("choose_backup_folder");
      const fix = getFixture("choose_backup_folder");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const res = await repo.chooseFolder();
      expect(
        commandReturnSchemas["choose_backup_folder"].safeParse(res).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith("choose_backup_folder");
    });

    it("choose_backup_file: abre seletor nativo de arquivo de backup", async () => {
      coveredCommands.add("choose_backup_file");
      const fix = getFixture("choose_backup_file");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const res = await repo.chooseFile();
      expect(
        commandReturnSchemas["choose_backup_file"].safeParse(res).success,
      ).toBe(true);
      expect(mockedInvoke).toHaveBeenCalledWith("choose_backup_file");
    });
  });

  describe("10. R3.4 — Prova de Fluxo de Recuperação com RecoveryKeyModal", () => {
    it("chama RecoveryKeyModal com o resultado do repositório (fixture real) e prova que a chave exibida é a que o Rust devolveu", async () => {
      const authRepo = new TauriAuthRepository();
      const recoverFixture = getFixture("recover_password");

      mockedInvoke.mockResolvedValueOnce(recoverFixture.sample_output);

      const repoResult = await authRepo.recoverPasswordWithKey({
        email: "diretor@someluz.com.br",
        recovery_key: "ER-OLD-KEY-1234",
        new_password: "NovaSenhaSegura#2026",
      });

      const expectedKeyFromRust = (
        recoverFixture.sample_output as { new_recovery_key: string }
      ).new_recovery_key;
      expect(repoResult.new_recovery_key).toBe(expectedKeyFromRust);

      const { container } = render(RecoveryKeyModal, {
        props: {
          open: true,
          recoveryKey: repoResult.new_recovery_key,
          onclose: () => {},
        },
      });

      const keyElement = container.querySelector(
        '[data-testid="new-recovery-key-display"]',
      );
      expect(keyElement).not.toBeNull();
      expect(keyElement?.textContent?.trim()).toBe(expectedKeyFromRust);
      expect(container.textContent).toContain(expectedKeyFromRust);
    });
  });

  describe("11. Tarefa 0.3 — Garantia Estrutural: Cobertura Integral dos 70 Comandos IPC", () => {
    it("FALHA se qualquer comando do registro COMMAND_REGISTRY (70 fixtures) não possuir teste de contrato de repositório", () => {
      const allFixtureCommands = Object.keys(fixtureModules)
        .map((p) => p.replace("./fixtures/contracts/", "").replace(".json", ""))
        .sort();

      expect(allFixtureCommands.length).toBe(70);

      const missingCommands: string[] = [];
      for (const cmd of allFixtureCommands) {
        if (!coveredCommands.has(cmd)) {
          missingCommands.push(cmd);
        }
      }

      expect(
        missingCommands,
        `Os seguintes comandos estão em COMMAND_REGISTRY / fixtures mas NÃO possuem teste de contrato em repositório: ${missingCommands.join(", ")}`,
      ).toEqual([]);

      expect(coveredCommands.size).toBe(70);
    });
  });
});
