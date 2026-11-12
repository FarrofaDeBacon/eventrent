import type { CustomerRepository } from "./customer-repository";
import { getSharedMockState } from "./mock-auth-repository";
import {
  PERMISSIONS,
  type CreateCustomerInput,
  type Customer,
  type CustomerAddress,
  type CustomerContact,
  type CustomerFilter,
  type CustomerPage,
  type CustomerWithDetails,
  type DocumentType,
  type UpdateCustomerInput,
} from "./types";
import {
  formatDocument,
  normalizeDocument,
  normalizeForSearch,
  validateCnpj,
  validateCpf,
  validateForeignDocument,
} from "$lib/utils/document";

interface MockCustomerStore {
  customers: CustomerWithDetails[];
  initialized: boolean;
}

const STORAGE_KEY = "eventrent:mock:customers:v1";

const memoryStore: MockCustomerStore = {
  customers: [],
  initialized: false,
};

function createSeedCustomers(tenantId: string): CustomerWithDetails[] {
  const now = new Date().toISOString();
  const seedSpecs: Array<{
    id: string;
    input: CreateCustomerInput;
  }> = [
    {
      id: "cust-seed-1",
      input: {
        person_type: "legal",
        name: "Produtora Horizonte Eventos Ltda",
        trade_name: "Horizonte Eventos",
        document_number: "11222333000181",
        state_registration: "110.042.490.114",
        municipal_registration: null,
        notes: "Cliente corporativo recorrente para festivais de verão.",
        contacts: [
          {
            name: "Mariana Albuquerque",
            email: "mariana@horizonte.com.br",
            phone: "(11) 98765-4321",
            role: "Produtora Executiva",
            is_primary: true,
          },
        ],
        addresses: [
          {
            label: "Sede",
            street: "Av. Paulista",
            number: "1578",
            complement: "Conjunto 402",
            neighborhood: "Bela Vista",
            city: "São Paulo",
            state: "SP",
            postal_code: "01310-200",
            is_primary: true,
          },
        ],
      },
    },
    {
      id: "cust-seed-2",
      input: {
        person_type: "individual",
        name: "Carlos Eduardo da Silva",
        trade_name: null,
        document_number: "52998224725",
        state_registration: null,
        municipal_registration: null,
        notes: "Técnico de som parceiro para eventos sociais.",
        contacts: [
          {
            name: "Carlos Eduardo",
            email: "carlos.silva@audio.br",
            phone: "(21) 99123-4567",
            role: "Titular",
            is_primary: true,
          },
        ],
        addresses: [
          {
            label: "Principal",
            street: "Rua das Laranjeiras",
            number: "320",
            complement: null,
            neighborhood: "Laranjeiras",
            city: "Rio de Janeiro",
            state: "RJ",
            postal_code: "22240-003",
            is_primary: true,
          },
        ],
      },
    },
  ];

  return seedSpecs.map(({ id, input }) => {
    const docType: DocumentType =
      input.document_type ??
      (input.person_type === "individual" ? "cpf" : "cnpj");
    const cleanDoc = input.document_number
      ? normalizeDocument(input.document_number)
      : null;
    const formattedDoc = cleanDoc
      ? formatDocument(cleanDoc, input.person_type)
      : "";
    const contactTerms = input.contacts
      .flatMap((c) => [c.name, c.email ?? "", c.phone ?? ""])
      .filter(Boolean);
    const searchText = [
      normalizeForSearch(input.name),
      input.trade_name ? normalizeForSearch(input.trade_name) : "",
      cleanDoc ? cleanDoc.toLowerCase() : "",
      formattedDoc.toLowerCase(),
      ...contactTerms.map((t) => normalizeForSearch(t)),
    ]
      .filter(Boolean)
      .join(" ");

    const customer: Customer = {
      id,
      tenant_id: tenantId,
      person_type: input.person_type,
      name: input.name,
      trade_name: input.trade_name ?? null,
      document_type: docType,
      document_number: cleanDoc,
      state_registration: input.state_registration ?? null,
      municipal_registration: input.municipal_registration ?? null,
      notes: input.notes ?? null,
      active: true,
      search_text: searchText,
      name_sort: normalizeForSearch(input.name),
      created_at: now,
      updated_at: now,
      deleted_at: null,
      created_by: "system",
    };

    const contacts: CustomerContact[] = input.contacts.map((c, idx) => ({
      id: `${id}-ct-${idx + 1}`,
      tenant_id: tenantId,
      customer_id: id,
      name: c.name,
      email: c.email ?? null,
      phone: c.phone ?? null,
      role: c.role ?? null,
      is_primary: c.is_primary,
      created_at: now,
      updated_at: now,
      deleted_at: null,
      created_by: "system",
    }));

    const addresses: CustomerAddress[] = input.addresses.map((a, idx) => ({
      id: `${id}-ad-${idx + 1}`,
      tenant_id: tenantId,
      customer_id: id,
      label: a.label,
      street: a.street,
      number: a.number,
      complement: a.complement ?? null,
      neighborhood: a.neighborhood,
      city: a.city,
      state: a.state,
      postal_code: a.postal_code,
      country: a.country ?? "BR",
      is_primary: a.is_primary,
      created_at: now,
      updated_at: now,
      deleted_at: null,
      created_by: "system",
    }));

    return { customer, contacts, addresses };
  });
}

function loadStore(tenantId: string): MockCustomerStore {
  if (!memoryStore.initialized) {
    if (typeof window !== "undefined" && window.localStorage) {
      const raw = window.localStorage.getItem(STORAGE_KEY);
      if (raw) {
        try {
          const parsed = JSON.parse(raw) as CustomerWithDetails[];
          if (Array.isArray(parsed)) {
            memoryStore.customers = parsed;
            memoryStore.initialized = true;
            return memoryStore;
          }
        } catch {
          // fallback para seed
        }
      }
    }
    memoryStore.customers = createSeedCustomers(tenantId);
    memoryStore.initialized = true;
    persistStore();
  }
  return memoryStore;
}

function persistStore(): void {
  if (typeof window !== "undefined" && window.localStorage) {
    try {
      window.localStorage.setItem(
        STORAGE_KEY,
        JSON.stringify(memoryStore.customers),
      );
    } catch {
      // ignora em caso de quota cheia no mock
    }
  }
}

export function resetMockCustomers(): void {
  memoryStore.customers = [];
  memoryStore.initialized = false;
  if (typeof window !== "undefined" && window.localStorage) {
    window.localStorage.removeItem(STORAGE_KEY);
  }
}

function requireMockCustomerPermission(permission: string) {
  const state = getSharedMockState();
  if (!state.session) {
    throw new Error("Sessão não autenticada.");
  }
  if (!state.session.permissions.includes(permission)) {
    throw new Error(`Permissão insuficiente: requer '${permission}'.`);
  }
  return { state, session: state.session };
}

function buildSearchText(
  input: CreateCustomerInput,
  cleanDoc: string | null,
): string {
  const formattedDoc = cleanDoc
    ? formatDocument(cleanDoc, input.person_type)
    : "";
  const contactTerms = input.contacts
    .flatMap((c) => [c.name, c.email ?? "", c.phone ?? ""])
    .filter(Boolean);
  return [
    normalizeForSearch(input.name),
    input.trade_name ? normalizeForSearch(input.trade_name) : "",
    cleanDoc ? cleanDoc.toLowerCase() : "",
    formattedDoc.toLowerCase(),
    ...contactTerms.map((t) => normalizeForSearch(t)),
  ]
    .filter(Boolean)
    .join(" ");
}

function validateInput(input: CreateCustomerInput): {
  docType: DocumentType;
  cleanDoc: string | null;
} {
  const name = input.name.trim();
  if (name.length < 2) {
    throw new Error(
      "O nome / razão social do cliente deve ter pelo menos 2 caracteres.",
    );
  }

  const docType: DocumentType =
    input.document_type ??
    (input.document_number
      ? input.person_type === "individual"
        ? "cpf"
        : "cnpj"
      : "none");
  const rawDoc = (input.document_number ?? "").trim();
  let cleanDoc: string | null = null;

  if (docType === "cpf") {
    if (input.person_type !== "individual") {
      throw new Error("Pessoa Jurídica não pode utilizar CPF.");
    }
    cleanDoc = normalizeDocument(rawDoc);
    if (!validateCpf(rawDoc)) {
      throw new Error("CPF inválido.");
    }
  } else if (docType === "cnpj") {
    if (input.person_type !== "legal") {
      throw new Error("Pessoa Física não pode utilizar CNPJ.");
    }
    cleanDoc = normalizeDocument(rawDoc);
    if (!validateCnpj(rawDoc)) {
      throw new Error("CNPJ inválido.");
    }
  } else if (docType === "foreign") {
    cleanDoc = normalizeDocument(rawDoc);
    if (!validateForeignDocument(rawDoc)) {
      throw new Error("Documento estrangeiro inválido (5 a 30 caracteres).");
    }
  } else if (rawDoc.length > 0) {
    throw new Error(
      "Quando o tipo de documento é 'none', o número do documento deve ficar vazio.",
    );
  }

  if (input.contacts.length === 0) {
    throw new Error("Cliente deve possuir pelo menos um contato.");
  }
  for (let i = 0; i < input.contacts.length; i++) {
    if (input.contacts[i].name.trim().length < 2) {
      throw new Error(`Nome do contato #${i + 1} é obrigatório.`);
    }
  }
  if (input.addresses.length === 0) {
    throw new Error("Cliente deve possuir pelo menos um endereço cadastrado.");
  }
  if (input.contacts.filter((c) => c.is_primary).length > 1) {
    throw new Error("Apenas um contato pode ser marcado como principal.");
  }
  if (input.addresses.filter((a) => a.is_primary).length > 1) {
    throw new Error("Apenas um endereço pode ser marcado como principal.");
  }

  return { docType, cleanDoc };
}

export class MockCustomerRepository implements CustomerRepository {
  async listCustomers(filter: CustomerFilter): Promise<CustomerPage> {
    const { session } = requireMockCustomerPermission(
      PERMISSIONS.CUSTOMER_READ,
    );
    const store = loadStore(session.tenant_id);

    let items = store.customers
      .map((entry) => entry.customer)
      .filter((c) => c.tenant_id === session.tenant_id && !c.deleted_at);

    if (filter.person_type) {
      items = items.filter((c) => c.person_type === filter.person_type);
    }

    if (filter.query && filter.query.trim()) {
      const words = normalizeForSearch(filter.query).split(" ").filter(Boolean);
      items = items.filter((c) =>
        words.every((w) => c.search_text.includes(w)),
      );
    }

    items.sort((a, b) => {
      const cmp = a.name_sort.localeCompare(b.name_sort);
      return cmp !== 0 ? cmp : a.id.localeCompare(b.id);
    });

    const totalCount = items.length;
    const limit = Math.min(Math.max(filter.limit ?? 50, 1), 200);

    if (filter.cursor) {
      try {
        const decoded = JSON.parse(atob(filter.cursor)) as {
          name_sort: string;
          id: string;
        };
        items = items.filter(
          (c) =>
            c.name_sort > decoded.name_sort ||
            (c.name_sort === decoded.name_sort && c.id > decoded.id),
        );
      } catch {
        throw new Error("Cursor de paginação inválido.");
      }
    }

    const hasMore = items.length > limit;
    const pageItems = items.slice(0, limit);
    const last = pageItems[pageItems.length - 1];
    const nextCursor =
      hasMore && last
        ? btoa(JSON.stringify({ name_sort: last.name_sort, id: last.id }))
        : null;

    return {
      items: pageItems.map((c) => ({ ...c })),
      next_cursor: nextCursor,
      has_more: hasMore,
      total_count: totalCount,
    };
  }

  async getCustomer(id: string): Promise<CustomerWithDetails> {
    const { session } = requireMockCustomerPermission(
      PERMISSIONS.CUSTOMER_READ,
    );
    const store = loadStore(session.tenant_id);
    const found = store.customers.find(
      (entry) =>
        entry.customer.id === id &&
        entry.customer.tenant_id === session.tenant_id &&
        !entry.customer.deleted_at,
    );
    if (!found) {
      throw new Error("Cliente não encontrado.");
    }
    return structuredClone(found);
  }

  async createCustomer(
    input: CreateCustomerInput,
  ): Promise<CustomerWithDetails> {
    const { session } = requireMockCustomerPermission(
      PERMISSIONS.CUSTOMER_MANAGE,
    );
    const store = loadStore(session.tenant_id);
    const { docType, cleanDoc } = validateInput(input);

    if (cleanDoc) {
      const duplicate = store.customers.some(
        (entry) =>
          entry.customer.tenant_id === session.tenant_id &&
          !entry.customer.deleted_at &&
          entry.customer.document_type === docType &&
          entry.customer.document_number === cleanDoc,
      );
      if (duplicate) {
        throw new Error(
          `Já existe um cliente ativo cadastrado com o documento '${formatDocument(cleanDoc, input.person_type)}'.`,
        );
      }
    }

    const now = new Date().toISOString();
    const id = `cust-${Date.now()}-${Math.floor(Math.random() * 10000)}`;
    const customer: Customer = {
      id,
      tenant_id: session.tenant_id,
      person_type: input.person_type,
      name: input.name.trim(),
      trade_name: input.trade_name?.trim() || null,
      document_type: docType,
      document_number: cleanDoc,
      state_registration: input.state_registration?.trim() || null,
      municipal_registration: input.municipal_registration?.trim() || null,
      notes: input.notes?.trim() || null,
      active: true,
      search_text: buildSearchText(input, cleanDoc),
      name_sort: normalizeForSearch(input.name),
      created_at: now,
      updated_at: now,
      deleted_at: null,
      created_by: session.user_id,
    };

    const contacts: CustomerContact[] = input.contacts.map((c, idx) => ({
      id: `${id}-ct-${idx + 1}`,
      tenant_id: session.tenant_id,
      customer_id: id,
      name: c.name.trim(),
      email: c.email?.trim() || null,
      phone: c.phone?.trim() || null,
      role: c.role?.trim() || null,
      is_primary: c.is_primary,
      created_at: now,
      updated_at: now,
      deleted_at: null,
      created_by: session.user_id,
    }));

    const addresses: CustomerAddress[] = input.addresses.map((a, idx) => ({
      id: `${id}-ad-${idx + 1}`,
      tenant_id: session.tenant_id,
      customer_id: id,
      label: a.label.trim(),
      street: a.street.trim(),
      number: a.number.trim(),
      complement: a.complement?.trim() || null,
      neighborhood: a.neighborhood.trim(),
      city: a.city.trim(),
      state: a.state.trim().toUpperCase(),
      postal_code: a.postal_code.trim(),
      country: a.country?.trim().toUpperCase() || "BR",
      is_primary: a.is_primary,
      created_at: now,
      updated_at: now,
      deleted_at: null,
      created_by: session.user_id,
    }));

    const created = { customer, contacts, addresses };
    store.customers.push(created);
    persistStore();
    return structuredClone(created);
  }

  async updateCustomer(
    id: string,
    input: UpdateCustomerInput,
  ): Promise<CustomerWithDetails> {
    const { session } = requireMockCustomerPermission(
      PERMISSIONS.CUSTOMER_MANAGE,
    );
    const store = loadStore(session.tenant_id);
    const { docType, cleanDoc } = validateInput(input);

    const idx = store.customers.findIndex(
      (entry) =>
        entry.customer.id === id &&
        entry.customer.tenant_id === session.tenant_id &&
        !entry.customer.deleted_at,
    );
    if (idx === -1) {
      throw new Error("Cliente não encontrado.");
    }

    if (cleanDoc) {
      const duplicate = store.customers.some(
        (entry) =>
          entry.customer.id !== id &&
          entry.customer.tenant_id === session.tenant_id &&
          !entry.customer.deleted_at &&
          entry.customer.document_type === docType &&
          entry.customer.document_number === cleanDoc,
      );
      if (duplicate) {
        throw new Error(
          `Já existe outro cliente ativo com o documento '${formatDocument(cleanDoc, input.person_type)}'.`,
        );
      }
    }

    const now = new Date().toISOString();
    const existing = store.customers[idx].customer;
    const customer: Customer = {
      ...existing,
      person_type: input.person_type,
      name: input.name.trim(),
      trade_name: input.trade_name?.trim() || null,
      document_type: docType,
      document_number: cleanDoc,
      state_registration: input.state_registration?.trim() || null,
      municipal_registration: input.municipal_registration?.trim() || null,
      notes: input.notes?.trim() || null,
      search_text: buildSearchText(input, cleanDoc),
      name_sort: normalizeForSearch(input.name),
      updated_at: now,
    };

    const prevContacts = store.customers[idx].contacts;
    const contacts: CustomerContact[] = input.contacts.map((c, cIdx) => {
      const existingCt = c.id
        ? prevContacts.find((p) => p.id === c.id && !p.deleted_at)
        : undefined;
      return {
        id: existingCt?.id ?? `${id}-ct-${Date.now()}-${cIdx + 1}`,
        tenant_id: session.tenant_id,
        customer_id: id,
        name: c.name.trim(),
        email: c.email?.trim() || null,
        phone: c.phone?.trim() || null,
        role: c.role?.trim() || null,
        is_primary: c.is_primary,
        created_at: existingCt?.created_at ?? now,
        updated_at: now,
        deleted_at: null,
        created_by: existingCt?.created_by ?? session.user_id,
      };
    });

    const prevAddresses = store.customers[idx].addresses;
    const addresses: CustomerAddress[] = input.addresses.map((a, aIdx) => {
      const existingAd = a.id
        ? prevAddresses.find((p) => p.id === a.id && !p.deleted_at)
        : undefined;
      return {
        id: existingAd?.id ?? `${id}-ad-${Date.now()}-${aIdx + 1}`,
        tenant_id: session.tenant_id,
        customer_id: id,
        label: a.label.trim(),
        street: a.street.trim(),
        number: a.number.trim(),
        complement: a.complement?.trim() || null,
        neighborhood: a.neighborhood.trim(),
        city: a.city.trim(),
        state: a.state.trim().toUpperCase(),
        postal_code: a.postal_code.trim(),
        country: a.country?.trim().toUpperCase() || "BR",
        is_primary: a.is_primary,
        created_at: existingAd?.created_at ?? now,
        updated_at: now,
        deleted_at: null,
        created_by: existingAd?.created_by ?? session.user_id,
      };
    });

    const updated = { customer, contacts, addresses };
    store.customers[idx] = updated;
    persistStore();
    return structuredClone(updated);
  }

  async deleteCustomer(id: string): Promise<void> {
    const { session } = requireMockCustomerPermission(
      PERMISSIONS.CUSTOMER_MANAGE,
    );
    const store = loadStore(session.tenant_id);
    const target = store.customers.find(
      (entry) =>
        entry.customer.id === id &&
        entry.customer.tenant_id === session.tenant_id &&
        !entry.customer.deleted_at,
    );
    if (!target) {
      throw new Error("Cliente não encontrado.");
    }
    target.customer.deleted_at = new Date().toISOString();
    target.customer.updated_at = target.customer.deleted_at;
    persistStore();
  }
}
