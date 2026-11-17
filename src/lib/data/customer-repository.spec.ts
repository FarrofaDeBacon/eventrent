import { beforeEach, describe, expect, it } from "vitest";
import { canAccessRoute } from "./index";
import {
  MockAuthRepository,
  resetSharedMockState,
} from "./mock-auth-repository";
import {
  MockCustomerRepository,
  resetMockCustomers,
} from "./mock-customer-repository";
import { MockUserRepository } from "./mock-user-repository";
import { ROLE_SLUGS } from "./types";

function generateValidCpf(base9: number[]): string {
  let s1 = 0;
  for (let i = 0; i < 9; i++) s1 += base9[i] * (10 - i);
  const r1 = s1 % 11;
  const d1 = r1 < 2 ? 0 : 11 - r1;

  let s2 = 0;
  for (let i = 0; i < 9; i++) s2 += base9[i] * (11 - i);
  s2 += d1 * 2;
  const r2 = s2 % 11;
  const d2 = r2 < 2 ? 0 : 11 - r2;

  return `${base9.join("")}${d1}${d2}`;
}

describe("CustomerRepository (CRUD, Busca, Paginação por Cursor e RBAC)", () => {
  let authRepo: MockAuthRepository;
  let userRepo: MockUserRepository;
  let customerRepo: MockCustomerRepository;

  beforeEach(async () => {
    resetSharedMockState();
    resetMockCustomers();
    authRepo = new MockAuthRepository();
    userRepo = new MockUserRepository();
    customerRepo = new MockCustomerRepository();

    await authRepo.completeInitialSetup({
      mode: "local",
      company_name: "Palco & Luz Ltda",
      owner_name: "Ana Proprietária",
      owner_email: "ana@palco.com.br",
      owner_password: "AnaPassword#2026",
    });
  });

  it("deve executar CRUD completo, busca sem acentos, filtro PF/PJ e impedir documento duplicado", async () => {
    const initialList = await customerRepo.listCustomers({});
    expect(initialList.items.length).toBeGreaterThanOrEqual(2);

    // Busca sem acentos e multi-palavra ("sao" / "carlos silva")
    const searchCarlos = await customerRepo.listCustomers({
      query: "carlos silva",
      person_type: "individual",
    });
    expect(searchCarlos.items).toHaveLength(1);
    expect(searchCarlos.items[0].name).toContain("Carlos Eduardo da Silva");

    // Cria novo cliente PF
    const created = await customerRepo.createCustomer({
      person_type: "individual",
      name: "João Pedro Vasconcelos",
      document_number: generateValidCpf([1, 2, 3, 4, 5, 6, 7, 8, 9]),
      notes: "Cliente VIP",
      contacts: [
        {
          name: "João Pedro",
          email: "joao@vip.com.br",
          phone: "(31) 99888-7766",
          role: "Diretor",
          is_primary: true,
        },
      ],
      addresses: [
        {
          label: "Escritório",
          street: "Av. Afonso Pena",
          number: "500",
          neighborhood: "Centro",
          city: "Belo Horizonte",
          state: "MG",
          postal_code: "30130-001",
          is_primary: true,
        },
      ],
    });
    expect(created.customer.name).toBe("João Pedro Vasconcelos");

    // Busca por telefone contendo apenas dígitos encontra o cliente
    const searchPhone = await customerRepo.listCustomers({
      query: "31998887766",
    });
    expect(searchPhone.items.length).toBeGreaterThanOrEqual(1);
    expect(
      searchPhone.items.some((c) => c.name === "João Pedro Vasconcelos"),
    ).toBe(true);

    // Busca por nome do contato encontra o cliente
    const searchContact = await customerRepo.listCustomers({
      query: "João Pedro",
    });
    expect(
      searchContact.items.some((c) => c.name === "João Pedro Vasconcelos"),
    ).toBe(true);

    // Validação de limites na consulta de busca: máx 100 caracteres e máx 8 palavras
    await expect(
      customerRepo.listCustomers({ query: "a".repeat(101) }),
    ).rejects.toThrow(/O termo de busca não pode exceder 100 caracteres/i);

    await expect(
      customerRepo.listCustomers({
        query: "um dois tres quatro cinco seis sete oito nove",
      }),
    ).rejects.toThrow(/O termo de busca não pode conter mais de 8 palavras/i);

    // Impede duplicidade de documento em cliente ativo ou inativo
    await expect(
      customerRepo.createCustomer({
        person_type: "individual",
        name: "Duplicado Teste",
        document_number: created.customer.document_number,
        contacts: [{ name: "Contato", is_primary: true }],
        addresses: [
          {
            label: "Principal",
            street: "Rua X",
            number: "1",
            neighborhood: "Centro",
            city: "São Paulo",
            state: "SP",
            postal_code: "01001-000",
            is_primary: true,
          },
        ],
      }),
    ).rejects.toThrow(/Já existe um cliente cadastrado com este documento/i);

    // Atualiza e consulta por ID
    const updated = await customerRepo.updateCustomer(created.customer.id, {
      person_type: "individual",
      name: "João Pedro Vasconcelos Atualizado",
      document_number: created.customer.document_number,
      expected_updated_at: created.customer.updated_at,
      contacts: [
        {
          name: "João Pedro",
          email: "jp@email.com",
          is_primary: true,
        },
      ],
      addresses: [
        {
          label: "Principal",
          street: "Av. Afonso Pena",
          number: "501",
          neighborhood: "Centro",
          city: "Belo Horizonte",
          state: "MG",
          postal_code: "30130-001",
          is_primary: true,
        },
      ],
    });
    expect(updated.customer.name).toBe("João Pedro Vasconcelos Atualizado");

    // Concorrência: update com expected_updated_at desatualizado deve falhar com conflito
    await expect(
      customerRepo.updateCustomer(created.customer.id, {
        person_type: "individual",
        name: "Tentativa Concorrente",
        expected_updated_at: "2020-01-01T00:00:00Z",
        contacts: [
          {
            name: "João",
            email: "j@ex.com",
            is_primary: true,
          },
        ],
        addresses: [],
      }),
    ).rejects.toThrow(/O cadastro foi alterado por outra pessoa/i);

    const loaded = await customerRepo.getCustomer(created.customer.id);
    expect(loaded.addresses[0].number).toBe("501");

    // Desativa e reativa (setCustomerActive)
    const deactivated = await customerRepo.setCustomerActive(
      created.customer.id,
      false,
    );
    expect(deactivated.customer.active).toBe(false);
    const reactivated = await customerRepo.setCustomerActive(
      created.customer.id,
      true,
    );
    expect(reactivated.customer.active).toBe(true);

    // Exclui (soft delete) e valida que não aparece mais nem permite consulta
    await customerRepo.deleteCustomer(created.customer.id);
    await expect(customerRepo.getCustomer(created.customer.id)).rejects.toThrow(
      /não encontrado/i,
    );
  });

  it("deve paginar por cursor determinístico sem pular ou duplicar itens", async () => {
    for (let i = 0; i < 6; i++) {
      await customerRepo.createCustomer({
        person_type: "individual",
        name: `Cliente Paginado ${i}`,
        document_number: generateValidCpf([2, 3, 4, 5, 6, 7, 8, i, 1]),
        contacts: [{ name: `Contato ${i}`, is_primary: true }],
        addresses: [
          {
            label: "Principal",
            street: "Rua Teste",
            number: `${i + 1}`,
            neighborhood: "Centro",
            city: "São Paulo",
            state: "SP",
            postal_code: "01001-000",
            is_primary: true,
          },
        ],
      });
    }

    const page1 = await customerRepo.listCustomers({ limit: 4 });
    expect(page1.items).toHaveLength(4);
    expect(page1.has_more).toBe(true);
    expect(page1.next_cursor).toBeTruthy();

    const page2 = await customerRepo.listCustomers({
      limit: 4,
      cursor: page1.next_cursor!,
    });
    expect(page2.items).toHaveLength(4);

    const ids1 = new Set(page1.items.map((c) => c.id));
    for (const item of page2.items) {
      expect(ids1.has(item.id)).toBe(false);
    }
  });

  it("deve aplicar RBAC para leitura (Financeiro), fornecedores (Estoque) e bloqueio completo (Técnico)", async () => {
    const roles = await userRepo.listRoles();
    const financeRole = roles.find((r) => r.slug === ROLE_SLUGS.FINANCE)!;
    const warehouseRole = roles.find((r) => r.slug === ROLE_SLUGS.WAREHOUSE)!;
    const techRole = roles.find((r) => r.slug === ROLE_SLUGS.TECHNICIAN)!;

    await userRepo.createUser({
      name: "Maria Financeiro",
      email: "maria@palco.com.br",
      password: "MariaPassword#2026",
      role_id: financeRole.id,
    });
    await userRepo.createUser({
      name: "Pedro Estoque",
      email: "pedro@palco.com.br",
      password: "PedroPassword#2026",
      role_id: warehouseRole.id,
    });
    await userRepo.createUser({
      name: "Lucas Técnico",
      email: "lucas@palco.com.br",
      password: "LucasPassword#2026",
      role_id: techRole.id,
    });

    // Financeiro pode ler clientes e gerenciar fornecedores, mas não criar cliente
    await authRepo.logout();
    const financeSession = await authRepo.login({
      email: "maria@palco.com.br",
      password: "MariaPassword#2026",
    });
    expect(canAccessRoute(financeSession, "/customers")).toBe(true);
    expect(canAccessRoute(financeSession, "/suppliers")).toBe(true);
    expect(canAccessRoute(financeSession, "/customers/new")).toBe(true);
    const listAsFinance = await customerRepo.listCustomers({});
    expect(listAsFinance.items.length).toBeGreaterThan(0);
    await expect(
      customerRepo.createCustomer({
        person_type: "individual",
        name: "Sem Permissão",
        document_number: "52998224725",
        contacts: [{ name: "C", is_primary: true }],
        addresses: [
          {
            label: "P",
            street: "R",
            number: "1",
            neighborhood: "B",
            city: "SP",
            state: "SP",
            postal_code: "01001-000",
            is_primary: true,
          },
        ],
      }),
    ).rejects.toThrow(/Permissão insuficiente/i);

    // Estoque não tem customer:read nem acesso à rota /customers, mas tem supplier:read e supplier:manage (/suppliers)
    await authRepo.logout();
    const warehouseSession = await authRepo.login({
      email: "pedro@palco.com.br",
      password: "PedroPassword#2026",
    });
    expect(canAccessRoute(warehouseSession, "/customers")).toBe(false);
    expect(canAccessRoute(warehouseSession, "/suppliers")).toBe(true);
    await expect(
      customerRepo.listCustomers({ role_filter: "customer" }),
    ).rejects.toThrow(/Permissão insuficiente/i);
    const supplierListAsWarehouse = await customerRepo.listCustomers({
      role_filter: "supplier",
    });
    expect(supplierListAsWarehouse.items).toEqual([]);

    // Técnico não possui customer:read nem supplier:read
    await authRepo.logout();
    const techSession = await authRepo.login({
      email: "lucas@palco.com.br",
      password: "LucasPassword#2026",
    });
    expect(canAccessRoute(techSession, "/customers")).toBe(false);
    expect(canAccessRoute(techSession, "/suppliers")).toBe(false);
    await expect(customerRepo.listCustomers({})).rejects.toThrow(
      /Permissão insuficiente/i,
    );
  });

  it("deve cadastrar fornecedor com categorias, terceiro com dois papéis e filtrar por role_filter", async () => {
    const pureSupplier = await customerRepo.createCustomer({
      person_type: "legal",
      name: "Subloca Audio Brasil Ltda",
      document_type: "none",
      document_number: null,
      is_customer: false,
      is_supplier: true,
      supplier_categories: ["sublocacao", "transporte"],
      contacts: [
        {
          name: "Pedro Subloca",
          email: "pedro@subloca.com.br",
          phone: "(11) 98888-1234",
          is_primary: true,
        },
      ],
      addresses: [],
    });
    expect(pureSupplier.customer.is_customer).toBe(false);
    expect(pureSupplier.customer.is_supplier).toBe(true);
    expect(pureSupplier.supplier_categories).toEqual([
      "sublocacao",
      "transporte",
    ]);

    const dualParty = await customerRepo.createCustomer({
      person_type: "legal",
      name: "Parceiro Híbrido Ltda",
      document_type: "none",
      document_number: null,
      is_customer: true,
      is_supplier: true,
      supplier_categories: ["manutencao"],
      contacts: [
        {
          name: "Carla Híbrida",
          email: "carla@hibrido.com.br",
          is_primary: true,
        },
      ],
      addresses: [],
    });

    const onlySuppliers = await customerRepo.listCustomers({
      role_filter: "supplier",
    });
    expect(onlySuppliers.items.map((c) => c.id)).toContain(
      pureSupplier.customer.id,
    );
    expect(onlySuppliers.items.map((c) => c.id)).toContain(
      dualParty.customer.id,
    );

    const onlyBoth = await customerRepo.listCustomers({ role_filter: "both" });
    expect(onlyBoth.items.map((c) => c.id)).toEqual([dualParty.customer.id]);

    const onlyCustomers = await customerRepo.listCustomers({
      role_filter: "customer",
    });
    expect(onlyCustomers.items.map((c) => c.id)).not.toContain(
      pureSupplier.customer.id,
    );
    expect(onlyCustomers.items.map((c) => c.id)).toContain(
      dualParty.customer.id,
    );
  });
});
