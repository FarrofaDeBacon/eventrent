# Relatório de Execução — Fase 1C (Tarefa 0 + Fornecedores + ADR-0012)

**Branch**: `fase-1c-fornecedores`  
**Base**: `main` (`0705d94`, após merge do PR #7)  
**Data**: 17/11/2026  
**Status**: Concluída (PR aberto para `main`, sem merge automático e sem criação de tag)

---

## 1. Resumo dos Commits (Um Commit por Item)

| Tarefa / Item | Descrição                                                                                                                                                                            | Commit                          |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------- |
| **0.1**       | Normalização e armazenamento de telefones em E.164 (`+55...` e `+DDI...`), busca por dígitos, atualização de máscara/Debug/Zod/mock                                                  | `5459000`                       |
| **0.2**       | `validate_email` rejeita qualquer caractere não-ASCII antes de `to_lowercase()` (teste com `U+212A`)                                                                                 | `4b34444`                       |
| **0.3**       | `create_customer` ignora IDs enviados pelo cliente em contatos e endereços (sempre gera UUIDs)                                                                                       | `e013856`, `8e2d687`, `e071100` |
| **0.4**       | Validação de nomes de usuário e proprietário (`create_user`, `update_user`, `owner_name`): 1 a 100 caracteres (`chars().count()`), sem controle, já aparados                         | `7017909`                       |
| **1.1**       | Migração `0006_suppliers.sql`: papéis `is_customer` / `is_supplier`, `CHECK`, tabela `customer_supplier_category` com FK composta multitenant e documentação em `02-modelo-dados.md` | `0b3653f`                       |
| **1.2**       | Permissões `supplier:read` e `supplier:manage` (`manage` implica `read`), perfis de sistema atualizados e `system-roles.json` regenerado                                             | `2cc200a`                       |
| **1.3**       | Regras de serviço por papel (`customer`, `supplier`, `both`, `all`), visibilidade/edição granular e implementação SQLite                                                             | `0b00c74`                       |
| **1.4**       | Frontend: abas Clientes/Fornecedores/Todos, rota `/suppliers`, papéis e chips de categoria no formulário, menu e `Ctrl+K` com RBAC e i18n pt-BR                                      | `5d4753c`, `b790c3f`            |
| **1.5**       | Segregação de chave PIX, dados bancários e condições de pagamento registrada em `docs/seguranca.md` (adiados para Fase 6)                                                            | `1e104e3`                       |
| **1.6**       | Testes de contrato do store SQLite, RBAC/serviços no Core, Vitest e Playwright E2E (`e2e/suppliers.spec.ts`)                                                                         | `ea685f6`, `6fdf9a0`, `0a8e146` |
| **2**         | Documento **ADR-0012** (`docs/decisions/0012-modelo-de-alocacao.md`) e atualização de `docs/05-roadmap.md` (sem implementação de código)                                             | `a1b566e`                       |
| **Entrega**   | Relatório final em `docs/relatorios/fase-1c.md`                                                                                                                                      | _Este commit_                   |

---

## 2. URLs Raw Fixadas no Hash do Commit (`a1b566e`)

- **Migração `0006`:** https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/a1b566e/crates/store-sqlite/migrations/0006_suppliers.sql
- **`validation.rs`:** https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/a1b566e/crates/core/src/domain/validation.rs
- **`services/mod.rs`:** https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/a1b566e/crates/core/src/services/mod.rs
- **`store/mod.rs`:** https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/a1b566e/crates/core/src/store/mod.rs
- **`permission.rs`:** https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/a1b566e/crates/core/src/domain/permission.rs
- **`system-roles.json`:** https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/a1b566e/src/lib/data/fixtures/system-roles.json
- **`ADR-0012`:** https://raw.githubusercontent.com/FarrofaDeBacon/eventrent/a1b566e/docs/decisions/0012-modelo-de-alocacao.md

---

## 3. Trechos Exatos Copiados do Código Real

### 3.1. Migração `0006_suppliers.sql` (`crates/store-sqlite/migrations/0006_suppliers.sql`, linhas 95–175 e 221–243)

```sql
CREATE TABLE customer (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    person_type TEXT NOT NULL CHECK (person_type IN ('individual', 'legal')),
    name TEXT NOT NULL,
    trade_name TEXT,
    document_type TEXT NOT NULL DEFAULT 'none' CHECK (document_type IN ('cpf', 'cnpj', 'foreign', 'none')),
    document_number TEXT,
    state_registration TEXT,
    municipal_registration TEXT,
    notes TEXT,
    active INTEGER NOT NULL DEFAULT 1,
    is_customer INTEGER NOT NULL DEFAULT 1,
    is_supplier INTEGER NOT NULL DEFAULT 0,
    search_text TEXT NOT NULL,
    name_sort TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT,
    CHECK (is_customer = 1 OR is_supplier = 1),
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES app_user(id) ON DELETE SET NULL,
    UNIQUE (tenant_id, id)
);
```

```sql
CREATE TABLE customer_supplier_category (
    id TEXT PRIMARY KEY NOT NULL,
    tenant_id TEXT NOT NULL,
    customer_id TEXT NOT NULL,
    category TEXT NOT NULL CHECK (category IN ('sublocacao', 'manutencao', 'transporte', 'compras', 'servicos', 'outros')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, customer_id) REFERENCES customer(tenant_id, id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES app_user(id) ON DELETE SET NULL
);
```

```sql
CREATE INDEX idx_customer_tenant_roles
    ON customer (tenant_id, is_customer, is_supplier)
    WHERE deleted_at IS NULL;
```

```sql
CREATE UNIQUE INDEX idx_customer_supplier_category_active
    ON customer_supplier_category (tenant_id, customer_id, category)
    WHERE deleted_at IS NULL;
```

### 3.2. Telefone E.164 e `validate_email` com checagem ASCII prévia (`crates/core/src/domain/validation.rs`, linhas 17–32 e 175–261)

```rust
pub fn validate_email(email: &str) -> CoreResult<String> {
    if !email.is_ascii() {
        return Err(CoreError::Validation(
            "O e-mail deve conter apenas caracteres ASCII.".to_string(),
        ));
    }

    if email.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(CoreError::Validation(
            "O e-mail não pode conter espaços ou caracteres de controle.".to_string(),
        ));
    }
```

```rust
pub fn normalize_and_validate_phone(phone: &str, country: Option<&str>) -> CoreResult<String> {
    let trimmed = phone.trim();
    if trimmed.is_empty() {
        return Err(CoreError::Validation(
            "Telefone não pode ser vazio.".to_string(),
        ));
    }

    if !trimmed
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '+' | '(' | ')' | '-' | '.' | ' '))
    {
        return Err(CoreError::Validation(
            "Telefone contém caracteres inválidos. Utilize apenas números, espaços e os símbolos +, -, (, ), .".to_string(),
        ));
    }

    // '+' só é permitido como primeiro caractere
    if trimmed.chars().skip(1).any(|c| c == '+') {
        return Err(CoreError::Validation(
            "Telefone inválido: o símbolo '+' só pode aparecer no início do número.".to_string(),
        ));
    }

    let starts_with_plus = trimmed.starts_with('+');
    let mut digits: String = trimmed.chars().filter(|c| c.is_ascii_digit()).collect();

    if starts_with_plus {
        if digits.starts_with("55") {
            digits = digits[2..].to_string();
            if digits.len() == 10 || digits.len() == 11 {
                return Ok(format!("+55{digits}"));
            }
            return Err(CoreError::Validation(
                "Telefone inválido: para o Brasil (+55) deve conter 10 ou 11 dígitos com DDD (ex: 11987654321).".to_string(),
            ));
        }

        if (8..=15).contains(&digits.len()) {
            return Ok(format!("+{digits}"));
        }
        return Err(CoreError::Validation(
            "Telefone internacional inválido: deve conter entre 8 e 15 dígitos.".to_string(),
        ));
    }

    let is_br = country
        .map(|c| c.eq_ignore_ascii_case("BR"))
        .unwrap_or(true);

    if is_br {
        if digits.starts_with("55") && (digits.len() == 12 || digits.len() == 13) {
            digits = digits[2..].to_string();
        }
        if digits.len() == 10 || digits.len() == 11 {
            Ok(format!("+55{digits}"))
        } else {
            Err(CoreError::Validation(
                "Telefone inválido: para o Brasil deve conter 10 ou 11 dígitos com DDD (ex: 11987654321).".to_string(),
            ))
        }
    } else if (8..=15).contains(&digits.len()) {
        Ok(format!("+{digits}"))
    } else {
        Err(CoreError::Validation(
            "Telefone inválido: para outros países deve conter entre 8 e 15 dígitos.".to_string(),
        ))
    }
}

/// Valida e normaliza o nome de usuário / proprietário (1 a 100 caracteres, sem caracteres de controle, já aparado).
pub fn validate_user_name(name: &str) -> CoreResult<String> {
    let trimmed = name.trim();
    if trimmed.chars().any(|c| c.is_control()) {
        return Err(CoreError::Validation(
            "O nome do usuário não pode conter caracteres de controle ou quebras de linha."
                .to_string(),
        ));
    }
    let char_count = trimmed.chars().count();
    if !(1..=100).contains(&char_count) {
        return Err(CoreError::Validation(
            "O nome do usuário deve conter entre 1 e 100 caracteres.".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}
```

---

## 4. Resultados dos Portões de Qualidade e Testes

- **`pnpm verify`:**
  1. `svelte-check`: `0` erros e `0` avisos.
  2. Lint & Cores: `0` cores literais em `30` componentes `.svelte`; Prettier e ESLint aprovados.
  3. Vitest (`pnpm test:unit`): `6` arquivos, **36/36** testes aprovados.
  4. Congelamento de migrações (`scripts/test-check-migrations.js`): **6/6** cenários aprovados (`0001`..`0005` preservadas; `0006_suppliers.sql` autorizada).
  5. `cargo fmt --check`: aprovado.
  6. `cargo clippy --all-targets -- -D warnings`: aprovado sem avisos.
  7. `cargo test --workspace`: **52/52** testes aprovados (`9` em `eventrent_app_lib`, `17` em `eventrent_core`, `26` em `eventrent_store_sqlite`).
- **Suíte E2E Playwright (`pnpm test:e2e`):**
  - **4/4** fluxos aprovados em Chromium headless (`smoke.spec.ts`, `branding.spec.ts`, `customers.spec.ts`, `suppliers.spec.ts`).

---

## 5. Declaração de Escopo

Todos os itens da Tarefa 0 (`0.1` a `0.4`), Tarefa 1 (`1.1` a `1.6`) e Tarefa 2 (`ADR-0012` e atualização de `docs/05-roadmap.md`) foram integralmente concluídos sem simplificações ou pendências não declaradas. Nenhum merge ou tag foi criado pelo agente.
