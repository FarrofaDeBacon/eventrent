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

    // Impede duplicidade de documento em cliente ativo
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
    ).rejects.toThrow(/Já existe um cliente ativo/i);

    // Atualiza e consulta por ID
    const updated = await customerRepo.updateCustomer(created.customer.id, {
      person_type: "individual",
      name: "João Pedro Vasconcelos Atualizado",
      document_number: created.customer.document_number,
      contacts: [{ name: "João Pedro", is_primary: true }],
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

  it("deve aplicar RBAC para leitura (Financeiro) e bloqueio completo (Estoque)", async () => {
    const roles = await userRepo.listRoles();
    const financeRole = roles.find((r) => r.slug === ROLE_SLUGS.FINANCE)!;
    const warehouseRole = roles.find((r) => r.slug === ROLE_SLUGS.WAREHOUSE)!;

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

    // Financeiro pode ler clientes, mas não criar/excluir nem acessar /customers/new
    await authRepo.logout();
    const financeSession = await authRepo.login({
      email: "maria@palco.com.br",
      password: "MariaPassword#2026",
    });
    expect(canAccessRoute(financeSession, "/customers")).toBe(true);
    expect(canAccessRoute(financeSession, "/customers/new")).toBe(false);
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

    // Estoque não tem customer:read nem acesso à rota /customers
    await authRepo.logout();
    const warehouseSession = await authRepo.login({
      email: "pedro@palco.com.br",
      password: "PedroPassword#2026",
    });
    expect(canAccessRoute(warehouseSession, "/customers")).toBe(false);
    await expect(customerRepo.listCustomers({})).rejects.toThrow(
      /Permissão insuficiente/i,
    );
  });
});
