import { describe, it, expect, beforeEach } from "vitest";
import {
  getAuthRepository,
  getUserRepository,
  getCustomerRepository,
  getVenueRepository,
  getBrandingRepository,
  getCatalogRepository,
  getSettingsRepository,
} from "./index";
import { resetSharedMockState } from "./mock-auth-repository";

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

describe("Contratos IPC entre Rust e TypeScript (Fixtures e Repositórios)", () => {
  let validRecoveryKey = "";

  beforeEach(async () => {
    resetSharedMockState();
    const authRepo = getAuthRepository();
    const setup = await authRepo.completeInitialSetup({
      company_name: "Showtime Eventos",
      owner_name: "Carlos Proprietário",
      owner_email: "carlos@eventrent.com.br",
      owner_password: "senhaSegura123!",
      mode: "local",
    });
    validRecoveryKey = setup.recovery_key;
  });

  it("deve existir o diretório de fixtures e conter todas as 70 fixtures de comandos", () => {
    const fixtureKeys = Object.keys(fixtureModules);
    expect(fixtureKeys.length).toBe(70);
  });

  it("cada fixture deve conter a estrutura canônica completa e sem campos vazios", () => {
    for (const [filePath, fixture] of Object.entries(fixtureModules)) {
      const expectedCommandName = filePath
        .replace("./fixtures/contracts/", "")
        .replace(".json", "");

      expect(fixture.command).toBe(expectedCommandName);
      expect(typeof fixture.is_public).toBe("boolean");
      expect(typeof fixture.has_return_value).toBe("boolean");
      expect(fixture.sample_errors).toBeDefined();

      if (fixture.has_return_value) {
        expect(fixture.sample_output).not.toBeNull();
      } else {
        expect(fixture.sample_output).toBeNull();
      }
    }
  });

  it("comandos críticos de recuperação e regeneração de chave DEVEM retornar objeto com new_recovery_key", () => {
    const recoverFixture =
      fixtureModules["./fixtures/contracts/recover_password.json"];
    expect(recoverFixture).toBeDefined();
    expect(recoverFixture.has_return_value).toBe(true);
    expect(recoverFixture.sample_output).toHaveProperty("new_recovery_key");
    const recOutput = recoverFixture.sample_output as {
      new_recovery_key: string;
    };
    expect(typeof recOutput.new_recovery_key).toBe("string");
    expect(recOutput.new_recovery_key.length).toBeGreaterThan(0);

    const regenFixture =
      fixtureModules["./fixtures/contracts/regenerate_recovery_key.json"];
    expect(regenFixture).toBeDefined();
    expect(regenFixture.has_return_value).toBe(true);
    expect(regenFixture.sample_output).toHaveProperty("new_recovery_key");
    const regOutput = regenFixture.sample_output as {
      new_recovery_key: string;
    };
    expect(typeof regOutput.new_recovery_key).toBe("string");
    expect(regOutput.new_recovery_key.length).toBeGreaterThan(0);
  });

  it("o repositório de autenticação no frontend NÃO deve tipar como Promise<void> os métodos com retorno", async () => {
    const authRepo = getAuthRepository();

    // getAppStatus
    const status = await authRepo.getAppStatus();
    expect(status).toBeDefined();
    expect(typeof status.has_owner).toBe("boolean");
    expect(status.config).toBeDefined();

    // getSession
    const session = await authRepo.getSession();
    expect(session === null || typeof session === "object").toBe(true);

    // recoverPasswordWithKey deve retornar RecoverPasswordResult com new_recovery_key
    const recoverResult = await authRepo.recoverPasswordWithKey({
      email: "carlos@eventrent.com.br",
      recovery_key: validRecoveryKey,
      new_password: "novaSenhaSegura123!",
    });
    expect(recoverResult).toBeDefined();
    expect(recoverResult.new_recovery_key).toBeDefined();
    expect(typeof recoverResult.new_recovery_key).toBe("string");

    // regenerateRecoveryKey deve retornar RegenerateRecoveryKeyResult com new_recovery_key
    const regenResult = await authRepo.regenerateRecoveryKey({
      current_password: "novaSenhaSegura123!",
    });
    expect(regenResult).toBeDefined();
    expect(regenResult.new_recovery_key).toBeDefined();
    expect(typeof regenResult.new_recovery_key).toBe("string");
  });

  it("os repositórios de domínio NÃO devem descartar retornos de comandos que produzem dados", async () => {
    const userRepo = getUserRepository();
    const brandingRepo = getBrandingRepository();
    const customerRepo = getCustomerRepository();
    const venueRepo = getVenueRepository();
    const catalogRepo = getCatalogRepository();
    const settingsRepo = getSettingsRepository();

    // Users
    const roles = await userRepo.listRoles();
    expect(Array.isArray(roles)).toBe(true);
    expect(roles.length).toBeGreaterThan(0);

    const users = await userRepo.listUsers();
    expect(Array.isArray(users)).toBe(true);

    // Branding
    const pubBranding = await brandingRepo.getPublicBranding();
    expect(pubBranding === null || typeof pubBranding === "object").toBe(true);

    // Customers
    const customers = await customerRepo.listCustomers({});
    expect(customers).toBeDefined();
    expect(Array.isArray(customers.items)).toBe(true);

    // Venues
    const venues = await venueRepo.list({});
    expect(venues).toBeDefined();
    expect(Array.isArray(venues.items)).toBe(true);

    // Settings
    const settings = await settingsRepo.getAllocationSettings();
    expect(settings).toBeDefined();
    expect(typeof settings.internal_transfer_minutes).toBe("number");

    // Catalog
    const brands = await catalogRepo.listBrands();
    expect(Array.isArray(brands)).toBe(true);

    const categoriesTree = await catalogRepo.listCategoriesTree();
    expect(Array.isArray(categoriesTree)).toBe(true);

    const models = await catalogRepo.listEquipmentModels({ limit: 50 });
    expect(models).toBeDefined();
    expect(Array.isArray(models.items)).toBe(true);
    expect(typeof models.total_count).toBe("number");
  });

  it("nenhuma fixture órfã deve existir além dos 62 comandos mapeados", () => {
    const knownCommands = [
      "get_app_status",
      "complete_initial_setup",
      "get_public_branding",
      "login",
      "logout",
      "get_current_session",
      "recover_password",
      "regenerate_recovery_key",
      "get_branding",
      "save_branding",
      "list_users",
      "create_user",
      "update_user",
      "set_user_active",
      "reset_user_password",
      "list_roles",
      "list_customers",
      "get_customer",
      "create_customer",
      "update_customer",
      "set_customer_active",
      "delete_customer",
      "add_customer_role",
      "list_venues",
      "get_venue",
      "create_venue",
      "update_venue",
      "set_venue_active",
      "delete_venue",
      "check_venue_duplicate_name_city",
      "get_allocation_settings",
      "update_allocation_settings",
      "list_brands",
      "create_brand",
      "update_brand",
      "delete_brand",
      "list_categories_tree",
      "get_category_effective_attributes",
      "create_category",
      "update_category",
      "delete_category",
      "create_attribute_definition",
      "update_attribute_definition",
      "delete_attribute_definition",
      "list_equipment_models",
      "get_equipment_model",
      "create_equipment_model",
      "update_equipment_model",
      "set_equipment_model_active",
      "delete_equipment_model",
      "load_suggested_catalog_categories",
      "list_model_media",
      "get_media_blob_bytes",
      "attach_model_photo",
      "attach_model_document",
      "update_model_media_metadata",
      "reorder_model_media",
      "set_primary_model_photo",
      "remove_model_media",
      "collect_unreferenced_media",
      "open_model_document_external",
      "save_model_document_as",
      "get_backup_status",
      "set_backup_settings",
      "run_backup_now",
      "list_backups",
      "verify_backup",
      "schedule_restore",
      "choose_backup_folder",
      "choose_backup_file",
    ];

    const fixtureKeys = Object.keys(fixtureModules);
    expect(fixtureKeys.length).toBe(knownCommands.length);

    for (const filePath of fixtureKeys) {
      const cmdName = filePath
        .replace("./fixtures/contracts/", "")
        .replace(".json", "");
      expect(knownCommands).toContain(cmdName);
    }
  });
});
