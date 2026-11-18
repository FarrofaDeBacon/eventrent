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
  type CustomerRole,
  type CustomerWithDetails,
  type DocumentType,
  type SessionInfo,
  type SupplierCategory,
  type UpdateCustomerInput,
} from "./types";
import {
  formatDocument,
  normalizeDocument,
  normalizeForSearch,
  normalizePhoneE164,
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
      is_customer: input.is_customer ?? true,
      is_supplier: input.is_supplier ?? false,
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
      phone: c.phone?.trim() ? normalizePhoneE164(c.phone) : null,
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

    return {
      customer,
      contacts,
      addresses,
      supplier_categories: input.supplier_categories ?? [],
    };
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

function requireMockSession(): SessionInfo {
  const state = getSharedMockState();
  if (!state.session) {
    throw new Error("Sessão não autenticada.");
  }
  return state.session;
}

function canReadCustomer(session: SessionInfo): boolean {
  return (
    session.permissions.includes(PERMISSIONS.CUSTOMER_READ) ||
    session.permissions.includes(PERMISSIONS.CUSTOMER_MANAGE)
  );
}

function canReadSupplier(session: SessionInfo): boolean {
  return (
    session.permissions.includes(PERMISSIONS.SUPPLIER_READ) ||
    session.permissions.includes(PERMISSIONS.SUPPLIER_MANAGE)
  );
}

function canManageCustomer(session: SessionInfo): boolean {
  return session.permissions.includes(PERMISSIONS.CUSTOMER_MANAGE);
}

function canManageSupplier(session: SessionInfo): boolean {
  return session.permissions.includes(PERMISSIONS.SUPPLIER_MANAGE);
}

function resolveValidatedRolesAndCategories(input: CreateCustomerInput): {
  isCustomer: boolean;
  isSupplier: boolean;
  categories: SupplierCategory[];
} {
  const isCustomer = input.is_customer ?? true;
  const isSupplier = input.is_supplier ?? false;
  if (!isCustomer && !isSupplier) {
    throw new Error(
      "O terceiro deve possuir pelo menos um papel: Cliente ou Fornecedor.",
    );
  }
  const categories = Array.from(new Set(input.supplier_categories ?? []));
  if (!isSupplier && categories.length > 0) {
    throw new Error(
      "Categorias de fornecedor só podem ser atribuídas quando o papel Fornecedor estiver marcado.",
    );
  }
  return { isCustomer, isSupplier, categories };
}

function buildSearchText(
  input: CreateCustomerInput,
  cleanDoc: string | null,
): string {
  const formattedDoc = cleanDoc
    ? formatDocument(cleanDoc, input.person_type)
    : "";
  const contactTerms = input.contacts
    .flatMap((c) => [
      c.name,
      c.email ?? "",
      c.phone ?? "",
      c.phone ? c.phone.replace(/\D/g, "") : "",
    ])
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
  if (input.contacts.filter((c) => c.is_primary).length > 1) {
    throw new Error("Apenas um contato pode ser marcado como principal.");
  }
  if (input.addresses.filter((a) => a.is_primary).length > 1) {
    throw new Error("Apenas um endereço pode ser marcado como principal.");
  }
  if (
    input.addresses.length > 0 &&
    !input.addresses.some((a) => a.is_primary)
  ) {
    input.addresses[0].is_primary = true;
  }

  return { docType, cleanDoc };
}

export class MockCustomerRepository implements CustomerRepository {
  async listCustomers(filter: CustomerFilter): Promise<CustomerPage> {
    const session = requireMockSession();
    const readCust = canReadCustomer(session);
    const readSupp = canReadSupplier(session);
    const roleFilter = filter.role_filter ?? "all";

    if (roleFilter === "customer" && !readCust) {
      throw new Error(
        `Permissão insuficiente: requer '${PERMISSIONS.CUSTOMER_READ}'.`,
      );
    }
    if (roleFilter === "supplier" && !readSupp) {
      throw new Error(
        `Permissão insuficiente: requer '${PERMISSIONS.SUPPLIER_READ}'.`,
      );
    }
    if (roleFilter === "both" && (!readCust || !readSupp)) {
      throw new Error(
        `Permissão insuficiente: requer '${PERMISSIONS.CUSTOMER_READ}' e '${PERMISSIONS.SUPPLIER_READ}'.`,
      );
    }
    if (roleFilter === "all" && !readCust && !readSupp) {
      throw new Error(
        `Permissão insuficiente: requer '${PERMISSIONS.CUSTOMER_READ}' ou '${PERMISSIONS.SUPPLIER_READ}'.`,
      );
    }

    const store = loadStore(session.tenant_id);

    if (filter.query) {
      const trimmed = filter.query.trim();
      if (trimmed.length > 100) {
        throw new Error("O termo de busca não pode exceder 100 caracteres.");
      }
      if (trimmed.split(/\s+/).filter(Boolean).length > 8) {
        throw new Error("O termo de busca não pode conter mais de 8 palavras.");
      }
    }

    let items = store.customers
      .map((entry) => entry.customer)
      .filter((c) => c.tenant_id === session.tenant_id && !c.deleted_at)
      .filter((c) => {
        if (roleFilter === "customer") return c.is_customer;
        if (roleFilter === "supplier") return c.is_supplier;
        if (roleFilter === "both") return c.is_customer && c.is_supplier;
        return (c.is_customer && readCust) || (c.is_supplier && readSupp);
      });

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
    const session = requireMockSession();
    const readCust = canReadCustomer(session);
    const readSupp = canReadSupplier(session);
    if (!readCust && !readSupp) {
      throw new Error(
        `Permissão insuficiente: requer '${PERMISSIONS.CUSTOMER_READ}' ou '${PERMISSIONS.SUPPLIER_READ}'.`,
      );
    }
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
    const visible =
      (found.customer.is_customer && readCust) ||
      (found.customer.is_supplier && readSupp);
    if (!visible) {
      throw new Error(
        found.customer.is_supplier
          ? `Permissão insuficiente: requer '${PERMISSIONS.SUPPLIER_READ}'.`
          : `Permissão insuficiente: requer '${PERMISSIONS.CUSTOMER_READ}'.`,
      );
    }
    return structuredClone(found);
  }

  async createCustomer(
    input: CreateCustomerInput,
  ): Promise<CustomerWithDetails> {
    const session = requireMockSession();
    const { isCustomer, isSupplier, categories } =
      resolveValidatedRolesAndCategories(input);
    if (isCustomer && !canManageCustomer(session)) {
      throw new Error(
        `Permissão insuficiente: requer '${PERMISSIONS.CUSTOMER_MANAGE}'.`,
      );
    }
    if (isSupplier && !canManageSupplier(session)) {
      throw new Error(
        `Permissão insuficiente: requer '${PERMISSIONS.SUPPLIER_MANAGE}'.`,
      );
    }
    const store = loadStore(session.tenant_id);
    const { docType, cleanDoc } = validateInput(input);

    if (cleanDoc) {
      const duplicate = store.customers.find(
        (entry) =>
          entry.customer.tenant_id === session.tenant_id &&
          !entry.customer.deleted_at &&
          entry.customer.document_type === docType &&
          entry.customer.document_number === cleanDoc,
      );
      if (duplicate) {
        const canReadExisting =
          (duplicate.customer.is_customer && canReadCustomer(session)) ||
          (duplicate.customer.is_supplier && canReadSupplier(session));
        if (canReadExisting) {
          throw new Error(
            `Já existe um cadastro com este documento (id: ${duplicate.customer.id}).`,
          );
        }
        throw new Error(
          "Este documento já está cadastrado. Se precisar usá-lo neste papel, peça a quem gerencia o cadastro existente.",
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
      is_customer: isCustomer,
      is_supplier: isSupplier,
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
      phone: c.phone?.trim() ? normalizePhoneE164(c.phone) : null,
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

    const created: CustomerWithDetails = {
      customer,
      contacts,
      addresses,
      supplier_categories: isSupplier ? categories : [],
    };
    store.customers.push(created);
    persistStore();
    return structuredClone(created);
  }

  async updateCustomer(
    id: string,
    input: UpdateCustomerInput,
  ): Promise<CustomerWithDetails> {
    const session = requireMockSession();
    const { isCustomer, isSupplier, categories } =
      resolveValidatedRolesAndCategories(input);
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
    const prevCustomer = store.customers[idx].customer;
    if (
      (isCustomer || prevCustomer.is_customer) &&
      !canManageCustomer(session)
    ) {
      throw new Error(
        `Permissão insuficiente: requer '${PERMISSIONS.CUSTOMER_MANAGE}'.`,
      );
    }
    if (
      (isSupplier || prevCustomer.is_supplier) &&
      !canManageSupplier(session)
    ) {
      throw new Error(
        `Permissão insuficiente: requer '${PERMISSIONS.SUPPLIER_MANAGE}'.`,
      );
    }

    if (
      input.expected_updated_at &&
      store.customers[idx].customer.updated_at !== input.expected_updated_at
    ) {
      throw new Error(
        "O cadastro foi alterado por outra pessoa. Recarregue e tente novamente.",
      );
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
          "Já existe um cliente cadastrado com este documento (ativo ou inativo).",
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
      is_customer: isCustomer,
      is_supplier: isSupplier,
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
        phone: c.phone?.trim() ? normalizePhoneE164(c.phone) : null,
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

    const updated: CustomerWithDetails = {
      customer,
      contacts,
      addresses,
      supplier_categories: isSupplier ? categories : [],
    };
    store.customers[idx] = updated;
    persistStore();
    return structuredClone(updated);
  }

  async setCustomerActive(
    id: string,
    active: boolean,
  ): Promise<CustomerWithDetails> {
    const session = requireMockSession();
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
    if (target.customer.is_customer && !canManageCustomer(session)) {
      throw new Error(
        `Permissão insuficiente: requer '${PERMISSIONS.CUSTOMER_MANAGE}'.`,
      );
    }
    if (target.customer.is_supplier && !canManageSupplier(session)) {
      throw new Error(
        `Permissão insuficiente: requer '${PERMISSIONS.SUPPLIER_MANAGE}'.`,
      );
    }
    target.customer.active = active;
    target.customer.updated_at = new Date().toISOString();
    persistStore();
    return structuredClone(target);
  }

  async deleteCustomer(id: string): Promise<void> {
    const session = requireMockSession();
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
    if (target.customer.is_customer && !canManageCustomer(session)) {
      throw new Error(
        `Permissão insuficiente: requer '${PERMISSIONS.CUSTOMER_MANAGE}'.`,
      );
    }
    if (target.customer.is_supplier && !canManageSupplier(session)) {
      throw new Error(
        `Permissão insuficiente: requer '${PERMISSIONS.SUPPLIER_MANAGE}'.`,
      );
    }
    target.customer.deleted_at = new Date().toISOString();
    target.customer.updated_at = target.customer.deleted_at;
    persistStore();
  }

  async addCustomerRole(
    id: string,
    role: CustomerRole,
  ): Promise<CustomerWithDetails> {
    const session = requireMockSession();
    const requiredPerm =
      role === "customer"
        ? PERMISSIONS.CUSTOMER_MANAGE
        : PERMISSIONS.SUPPLIER_MANAGE;
    if (!session.permissions.includes(requiredPerm)) {
      throw new Error(`Permissão insuficiente: requer '${requiredPerm}'.`);
    }

    const store = loadStore(session.tenant_id);
    const target = store.customers.find(
      (entry) =>
        entry.customer.id === id &&
        entry.customer.tenant_id === session.tenant_id &&
        !entry.customer.deleted_at,
    );
    if (!target) {
      throw new Error("Cadastro não encontrado.");
    }

    const already =
      role === "customer"
        ? target.customer.is_customer
        : target.customer.is_supplier;
    if (already) {
      return structuredClone(target);
    }

    if (role === "customer") {
      target.customer.is_customer = true;
    } else {
      target.customer.is_supplier = true;
    }
    target.customer.updated_at = new Date().toISOString();
    persistStore();
    return structuredClone(target);
  }
}
