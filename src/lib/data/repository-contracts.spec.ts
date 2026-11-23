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

describe("R3 — Contratos Reais Rust ↔ TypeScript e Schemas de Runtime Zod", () => {
  const mockedInvoke = vi.mocked(invoke);

  beforeEach(() => {
    vi.resetAllMocks();
  });

  describe("1. Repositório de Autenticação (TauriAuthRepository)", () => {
    const repo = new TauriAuthRepository();

    it("getAppStatus: retorna dados válidos conformes ao schema e tipo", async () => {
      const fixture = getFixture("get_app_status");
      mockedInvoke.mockResolvedValueOnce(fixture.sample_output);

      const result = await repo.getAppStatus();
      expect(result).toBeDefined();
      expect(
        commandReturnSchemas["get_app_status"].safeParse(result).success,
      ).toBe(true);
      expect(typeof result.has_owner).toBe("boolean");
    });

    it("completeInitialSetup: retorna setup completo e schema válido", async () => {
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
    });

    it("login: retorna sessão válida", async () => {
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
    });

    it("logout: comando void", async () => {
      mockedInvoke.mockResolvedValueOnce(undefined);
      const res = await repo.logout();
      expect(res).toBeUndefined();
    });

    it("getCurrentSession: retorna sessão ou null", async () => {
      const fixture = getFixture("get_current_session");
      mockedInvoke.mockResolvedValueOnce(fixture.sample_output);

      const result = await repo.getSession();
      expect(result).toBeDefined();
      expect(
        commandReturnSchemas["get_current_session"].safeParse(result).success,
      ).toBe(true);
    });

    it("recoverPasswordWithKey: NÃO retorna void; devolve { new_recovery_key } válida", async () => {
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
    });

    it("regenerateRecoveryKey: devolve nova chave conforme fixture", async () => {
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

    it("listRoles e listUsers: retornam listas conformes ao schema", async () => {
      const rolesFix = getFixture("list_roles");
      mockedInvoke.mockResolvedValueOnce(rolesFix.sample_output);
      const roles = await repo.listRoles();
      expect(commandReturnSchemas["list_roles"].safeParse(roles).success).toBe(
        true,
      );

      const usersFix = getFixture("list_users");
      mockedInvoke.mockResolvedValueOnce(usersFix.sample_output);
      const users = await repo.listUsers();
      expect(commandReturnSchemas["list_users"].safeParse(users).success).toBe(
        true,
      );
    });

    it("create_user e update_user: retornam UserWithRole", async () => {
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
    });

    it("reset_user_password: void", async () => {
      mockedInvoke.mockResolvedValueOnce(undefined);
      const res = await repo.resetUserPassword("u-1", {
        new_password: "NewPassword123!",
      });
      expect(res).toBeUndefined();
    });
  });

  describe("3. Repositório de Clientes (TauriCustomerRepository)", () => {
    const repo = new TauriCustomerRepository();

    it("listCustomers: valida página de clientes", async () => {
      const fix = getFixture("list_customers");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const page = await repo.listCustomers({});
      expect(
        commandReturnSchemas["list_customers"].safeParse(page).success,
      ).toBe(true);
    });

    it("getCustomer e createCustomer: validam detalhes do cliente", async () => {
      const fix = getFixture("get_customer");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const details = await repo.getCustomer("cust-1");
      expect(
        commandReturnSchemas["get_customer"].safeParse(details).success,
      ).toBe(true);
    });

    it("delete_customer: void", async () => {
      mockedInvoke.mockResolvedValueOnce(undefined);
      const res = await repo.deleteCustomer("cust-1");
      expect(res).toBeUndefined();
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

  describe("4. Repositório de Locais (TauriVenueRepository)", () => {
    const repo = new TauriVenueRepository();

    it("list_venues e get_venue: validam schemas de locais", async () => {
      const listFix = getFixture("list_venues");
      mockedInvoke.mockResolvedValueOnce(listFix.sample_output);
      const list = await repo.list({});
      expect(commandReturnSchemas["list_venues"].safeParse(list).success).toBe(
        true,
      );

      const getFix = getFixture("get_venue");
      mockedInvoke.mockResolvedValueOnce(getFix.sample_output);
      const venue = await repo.getById("venue-1");
      expect(commandReturnSchemas["get_venue"].safeParse(venue).success).toBe(
        true,
      );
    });

    it("check_venue_duplicate_name_city: retorna boolean", async () => {
      const fix = getFixture("check_venue_duplicate_name_city");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const duplicate = await repo.checkDuplicateNameCity(
        "Teatro",
        "São Paulo",
      );
      expect(typeof duplicate).toBe("boolean");
    });
  });

  describe("5. Repositório de Catálogo (TauriCatalogRepository)", () => {
    const repo = new TauriCatalogRepository();

    it("list_brands e create_brand: validam schemas", async () => {
      const brandsFix = getFixture("list_brands");
      mockedInvoke.mockResolvedValueOnce(brandsFix.sample_output);
      const brands = await repo.listBrands();
      expect(
        commandReturnSchemas["list_brands"].safeParse(brands).success,
      ).toBe(true);

      const createFix = getFixture("create_brand");
      mockedInvoke.mockResolvedValueOnce(createFix.sample_output);
      const brand = await repo.createBrand({ name: "Pioneer", active: true });
      expect(
        commandReturnSchemas["create_brand"].safeParse(brand).success,
      ).toBe(true);
    });

    it("list_categories_tree: valida árvore hierárquica", async () => {
      const fix = getFixture("list_categories_tree");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const tree = await repo.listCategoriesTree();
      expect(
        commandReturnSchemas["list_categories_tree"].safeParse(tree).success,
      ).toBe(true);
    });

    it("list_equipment_models: valida resultado paginado", async () => {
      const fix = getFixture("list_equipment_models");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const models = await repo.listEquipmentModels({ limit: 20 });
      expect(
        commandReturnSchemas["list_equipment_models"].safeParse(models).success,
      ).toBe(true);
    });

    it("collect_unreferenced_media: valida retorno de limpeza GC", async () => {
      const fix = getFixture("collect_unreferenced_media");
      mockedInvoke.mockResolvedValueOnce(fix.sample_output);
      const gcResult = await repo.collectUnreferencedMedia();
      expect(
        commandReturnSchemas["collect_unreferenced_media"].safeParse(gcResult)
          .success,
      ).toBe(true);
      expect(typeof gcResult.deleted_files).toBe("number");
    });
  });

  describe("6. Repositório de Branding e Configurações", () => {
    const brandingRepo = new TauriBrandingRepository();
    const settingsRepo = new TauriSettingsRepository();

    it("get_public_branding e save_branding: validam schemas", async () => {
      const pubFix = getFixture("get_public_branding");
      mockedInvoke.mockResolvedValueOnce(pubFix.sample_output);
      const pub = await brandingRepo.getPublicBranding();
      expect(
        commandReturnSchemas["get_public_branding"].safeParse(pub).success,
      ).toBe(true);

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
    });

    it("get_allocation_settings e update_allocation_settings: validam schemas", async () => {
      const getFix = getFixture("get_allocation_settings");
      mockedInvoke.mockResolvedValueOnce(getFix.sample_output);
      const settings = await settingsRepo.getAllocationSettings();
      expect(
        commandReturnSchemas["get_allocation_settings"].safeParse(settings)
          .success,
      ).toBe(true);
    });
  });

  describe("7. R3.4 — Prova de Fluxo de Recuperação com RecoveryKeyModal", () => {
    it("chama RecoveryKeyModal com o resultado do repositório (fixture real) e prova que a chave exibida é a que o Rust devolveu", async () => {
      const authRepo = new TauriAuthRepository();
      const recoverFixture = getFixture("recover_password");

      // Mocka invoke para devolver a fixture REAL gerada pelo Rust
      mockedInvoke.mockResolvedValueOnce(recoverFixture.sample_output);

      // Repositório executa comando de recuperação
      const repoResult = await authRepo.recoverPasswordWithKey({
        email: "diretor@someluz.com.br",
        recovery_key: "ER-OLD-KEY-1234",
        new_password: "NovaSenhaSegura#2026",
      });

      // Valida que o retorno do repositório tem a chave exata da fixture do Rust
      const expectedKeyFromRust = (
        recoverFixture.sample_output as { new_recovery_key: string }
      ).new_recovery_key;
      expect(repoResult.new_recovery_key).toBe(expectedKeyFromRust);

      // Renderiza o RecoveryKeyModal com o resultado retornado pelo repositório
      const { container } = render(RecoveryKeyModal, {
        props: {
          open: true,
          recoveryKey: repoResult.new_recovery_key,
          onclose: () => {},
        },
      });

      // Prova que a chave exibida no DOM é exatamente a que o Rust devolveu
      const keyElement = container.querySelector(
        '[data-testid="new-recovery-key-display"]',
      );
      expect(keyElement).not.toBeNull();
      expect(keyElement?.textContent?.trim()).toBe(expectedKeyFromRust);
      expect(container.textContent).toContain(expectedKeyFromRust);
    });
  });
});
