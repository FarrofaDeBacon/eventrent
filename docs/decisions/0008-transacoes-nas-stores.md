# ADR-0008: Proposta de Transações nas Stores (Atomicidade Multi-Store)

## Status

**Proposta** (Aguardando aprovação para implementação na fase apropriada)

## Contexto

Atualmente, `crates/core` consome persistência via traits assíncronas desacopladas (`TenantStore`, `BrandingStore`, `RoleStore`, `UserStore`, `AuditStore`) agregadas em uma `StoreBundle`.

Entretanto, fluxos de negócio críticos realizam múltiplas escritas correlacionadas sem garantia de atomicidade ACID:

1. **Configuração Inicial (`complete_initial_setup`)**: Cria o tenant, salva branding inicial, cria o usuário proprietário e gera evento de auditoria. Caso a criação do usuário falhe, o tenant já persiste gravado.
2. **Criação/Edição de Usuário e Segurança**: Criação de usuário seguida por inserção em `audit_log`. Se o sistema cair entre as duas chamadas, perde-se a rastreabilidade do evento.
3. **Motor de Disponibilidade de Equipamentos (Fase 3)**: O requisito central da Fase 3 exige _“verificar disponibilidade de estoque e gravar a reserva na mesma transação”_, prevenindo race conditions e overbooking sob concorrência (especialmente no modo nuvem/PostgreSQL da Fase 2.5).

Com as traits atuais operando diretamente contra pools de conexão individuais, não há como compor operações de diferentes stores em uma única transação atômica gerenciada pelo serviço do `core`.

---

## Opções Analisadas

### Opção 1: Unit of Work (UoW) com Stores Transacionais (`TransactionManager`)

Criação de uma abstração de transação no `crates/core`:

```rust
#[async_trait]
pub trait TransactionManager: Send + Sync {
    async fn begin_transaction(&self) -> Result<Box<dyn TxStoreBundle>>;
}

#[async_trait]
pub trait TxStoreBundle: Send + Sync {
    fn tenants(&self) -> &dyn TenantStore;
    fn branding(&self) -> &dyn BrandingStore;
    fn roles(&self) -> &dyn RoleStore;
    fn users(&self) -> &dyn UserStore;
    fn audit(&self) -> &dyn AuditStore;

    async fn commit(self: Box<Self>) -> Result<()>;
    async fn rollback(self: Box<Self>) -> Result<()>;
}
```

- **Prós**:
  - **Preservação das Traits**: As traits existentes (`UserStore`, `TenantStore`, etc.) continuam intactas. A implementação SQLite/Postgres simplesmente instancia implementações que usam `sqlx::Transaction<'_, DB>` em vez de `sqlx::Pool<DB>`.
  - **Flexibilidade Total**: O serviço orquestra a lógica de negócio, decidindo explicitamente a fronteira transacional.
  - **Essencial para a Fase 3**: Permite que o motor de disponibilidade consulte itens e insira locações sob o mesmo isolamento transacional (`SERIALIZABLE` ou `REPEATABLE READ`).
- **Contras**:
  - Exige gerenciamento cuidadoso do ciclo de vida da transação (garantir rollback via RAII/`Drop` se o commit não for invocado).
  - Custo mínimo de dispatch dinâmico (`Box<dyn TxStoreBundle>`).

---

### Opção 2: Callback Transacional de Orquestração (`run_in_transaction`)

Implementação de um runner no `crates/core` que recebe um closure assíncrono:

```rust
#[async_trait]
pub trait TransactionRunner: Send + Sync {
    async fn run_in_tx<F, T>(&self, f: F) -> Result<T>
    where
        F: for<'a> FnOnce(&'a StoreBundle) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>> + Send;
}
```

- **Prós**:
  - **Segurança de Commit/Rollback**: `commit` ocorre automaticamente se o closure retornar `Ok(T)`, e `rollback` imediato se retornar `Err` ou ocorrer pânico.
  - Não há risco do desenvolvedor esquecer de finalizar a transação.
- **Contras**:
  - Tipagem complexa em Rust assíncrono: lifetimes de Higher-Ranked Trait Bounds (`for<'a>`) e necessidade de `Pin<Box<dyn Future>>`.
  - Menor flexibilidade quando a transação precisa ser passada entre métodos auxiliares privados.

---

### Opção 3: Operações Compostas no Store (Coarse-Grained / Fachadas por Agregado)

Evitar abstrações transacionais no `core` e criar métodos específicos nas traits que embutem a transação internamente no store (ex: `complete_initial_setup_atomic(...)`, `reserve_items_if_available(...)`).

- **Prós**:
  - Zero alteração na infraestrutura atual de traits do `core`.
  - Sem closures assíncronas nem structs de Unit of Work.
- **Contras**:
  - **Acoplamento e Violação de SRP**: Empurra regras de orquestração de domínio para dentro do driver de persistência SQL.
  - **Inflexibilidade**: Cada nova combinação de escrita exige criar um método gigante no store.
  - Inviável para regras complexas da Fase 3 (onde consultas de disponibilidade dependem de regras dinâmicas de catálogo, kits, períodos de locação e buffers operacionais).

---

## Comparativo: SQLite vs PostgreSQL

| Critério                        | SQLite (`crates/store-sqlite`)                                                                              | PostgreSQL (`crates/store-postgres` - Fase 2.5)                                                   |
| :------------------------------ | :---------------------------------------------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------ |
| **Concorrência de Escrita**     | Um único escritor simultâneo por arquivo (`BEGIN IMMEDIATE`). Transações longas podem causar `SQLITE_BUSY`. | Concorrência multi-conexão com MVCC (Postgres suporta múltiplos escritores em linhas diferentes). |
| **Níveis de Isolamento**        | `SERIALIZABLE` inerente no modo WAL para escritores.                                                        | Configurável (`READ COMMITTED`, `REPEATABLE READ`, `SERIALIZABLE`).                               |
| **Compatibilidade com Opção 1** | `sqlx::Transaction<'_, Sqlite>` implementa perfeitamente as traits dos stores.                              | `sqlx::Transaction<'_, Postgres>` implementa a mesma interface com suporte nativo a savepoints.   |

---

## Impacto nas Traits Atuais

- As assinaturas de consulta e escrita individuais em `TenantStore`, `UserStore`, etc., **não mudam**.
- A struct `StoreBundle` em `crates/core` passa a conter opcionalmente uma referência a `Arc<dyn TransactionManager>` (ou `StoreBundle` é a visão de leitura/escrita padrão fora de tx e `TxStoreBundle` a visão dentro de tx).
- Nenhuma dependência externa nova é necessária no `crates/core` (continua sem `sqlx`).

---

## Recomendação

Recomenda-se a **Opção 1 (Unit of Work com `TransactionManager` / `TxStoreBundle`)**, combinada com um helper ergonômico no core para garantir rollback automático em caso de erro.

Essa abordagem:

1. Mantém `crates/core` 100% puro e agnóstico de SQL.
2. Resolve de forma limpa a atomicidade do setup inicial e auditoria.
3. Prepara arquiteturalmente o sistema para o motor de disponibilidade e reservas em tempo real da Fase 3, atendendo tanto o SQLite local quanto o PostgreSQL do servidor Axum (Fase 2.5).
