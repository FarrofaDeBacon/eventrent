# ADR-0008: Transações nas Stores (Atomicidade Multi-Store e run_in_tx)

## Status

**Aceito** (Decisão da Diretoria de Engenharia na Fase 1B.0)

## Contexto

Atualmente, `crates/core` consome persistência via traits assíncronas desacopladas (`TenantStore`, `BrandingStore`, `RoleStore`, `UserStore`, `AuditStore`) agregadas em uma `StoreBundle`.

Entretanto, fluxos de negócio críticos realizam múltiplas escritas correlacionadas sem garantia de atomicidade ACID:

1. **Configuração Inicial (`complete_initial_setup`)**: Cria o tenant, salva branding inicial, cria o usuário proprietário e gera evento de auditoria. Caso a criação do usuário falhasse, o tenant já persistia gravado.
2. **Criação/Edição de Usuário e Segurança**: Criação de usuário seguida por inserção em `audit_log`. Se o sistema caísse entre as duas chamadas, perdia-se a rastreabilidade do evento.
3. **Motor de Disponibilidade de Equipamentos (Fase 3)**: O requisito central da Fase 3 exige _“verificar disponibilidade de estoque e gravar a reserva na mesma transação”_, prevenindo race conditions e overbooking sob concorrência (especialmente no modo nuvem/PostgreSQL da Fase 2.5).

Com as traits originais operando diretamente contra pools de conexão individuais, não havia como compor operações de diferentes stores em uma única transação atômica gerenciada pelo serviço do `core`.

---

## Decisão

Adotou-se a **Opção 1 (Unit of Work com `TransactionManager` / `TxStoreBundle`)**, complementada pela função genérica de orquestração **`run_in_tx` no `crates/core`** (como função livre, não método de trait, garantindo total _object-safety_).

### 1. Modelo no Core (`crates/core`)

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

pub type TxBoxFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

pub async fn run_in_tx<F, T>(
    manager: &dyn TransactionManager,
    policy: &TransactionPolicy,
    f: F,
) -> Result<T>
where
    F: for<'a> Fn(&'a dyn TxStoreBundle) -> TxBoxFuture<'a, T>;
```

**Garantias do `run_in_tx`:**

- **Commit automático**: Ao retornar `Ok(T)`, invoca `commit().await`.
- **Rollback automático**: Ao retornar `Err(E)` ou ocorrer pânico na execução do closure (RAII / `Drop`), a transação é abortada e descartada com segurança.
- **Retry com Backoff Exponencial**: Quando o erro for classificado como `CoreError::TransientConflict(String)`, novas tentativas imediatas com backoff exponencial (padrão: até 3 tentativas) são executadas antes de propagar o erro.

---

### 2. Implementação no SQLite (`crates/store-sqlite`)

1. **Abstração de Executor (`SqliteExecutor`)**:
   Implementado como `enum SqliteExecutor { Pool(DbPool), Tx(Arc<tokio::sync::Mutex<Option<sqlx::Transaction<'static, Sqlite>>>>) }`.
   Todas as stores (`TenantStore`, `BrandingStore`, `RoleStore`, `UserStore`, `AuditStore`) executam suas queries via `self.executor` (`fetch_optional`, `fetch_one`, `fetch_all`, `execute`).
   **Benefício fundamental**: Zero duplicação de SQL ou bindings de parâmetros entre operações comuns no pool e operações dentro de transação.

2. **Transações com `BEGIN IMMEDIATE`**:
   No SQLite, `BEGIN` padrão é diferido (`DEFERRED`). Se duas conexões realizam leitura e depois tentam elevar para escrita simultaneamente, ocorre deadlock ou erro `database is locked`.
   O `SqliteTxManager` inicia conexões transacionais estritamente com `self.pool.begin_with("BEGIN IMMEDIATE")`, obtendo a trava de escrita imediatamente antes de qualquer leitura ou escrita.

3. **Tempo Limite de Espera (`busy_timeout`)**:
   Tanto `init_db` quanto `init_memory_db` configuram explicitamente `.busy_timeout(std::time::Duration::from_secs(5))` no driver do SQLite, concedendo até 5 segundos para que transações concorrentes concluam antes de devolver erro de lock.

4. **Mapeamento de Conflitos Transitórios**:
   Erros de SQLite com códigos 5 (`SQLITE_BUSY`), 6 (`SQLITE_LOCKED`), 261 (`SQLITE_BUSY_RECOVERY`), 517 (`SQLITE_BUSY_SNAPSHOT`), mensagens contendo `busy`/`locked` ou `PoolTimedOut` são mapeados como `CoreError::TransientConflict(msg)`, acionando a esteira de retry do `run_in_tx`.

5. **Tratamento no Tauri**:
   O Tauri mapeia `CoreError::TransientConflict` para mensagens amigáveis ao usuário final indicando que o sistema está processando concorrência e que uma nova tentativa em instantes terá sucesso.

---

### 3. Considerações Críticas para PostgreSQL (Fase 2.5 e Fase 3)

No PostgreSQL (arquitetura cliente-servidor com MVCC):

- O nível `REPEATABLE READ` sozinho **não é suficiente** para evitar o problema clássico de verificação-e-gravação (_check-then-act_) em reservas simultâneas de estoque. Em `REPEATABLE READ`, duas transações concorrentes que leem a disponibilidade de um item simultaneamente verão o mesmo saldo positivo; ambas gravarão a reserva e comitarão, gerando **overbooking**.
- Para a Fase 3 (Motor de Disponibilidade), será obrigatório adotar uma das seguintes estratégias suportadas pela arquitetura:
  1. **Trava explícita de linha**: `SELECT ... FOR UPDATE` nas linhas dos equipamentos/kits sendo reservados.
  2. **Restrições de Exclusão**: `EXCLUDE USING gist` com ranges temporais (`tsrange`) para impedir sobreposição física no banco.
  3. **Nível `SERIALIZABLE` estrito**: Com a infraestrutura do `run_in_tx` já provendo captura de conflitos de serialização (código `40001` no Postgres) e novas tentativas com backoff exponencial.

---

### 4. Cuidados de Auditoria e Persistência de Tentativas Falhas

Em operações de autenticação (`login` e `recover_password`):

- O incremento do contador de tentativas falhas e a aplicação de bloqueio temporário por brute-force **devem ser comitados no banco antes de devolver o erro ao chamador**.
- Se a transação fosse abortada (rollback) pelo retorno do erro `CoreError::InvalidCredentials`, o contador de falhas seria desfeito, neutralizando a proteção contra força bruta. Portanto, a gravação da falha e da auditoria é finalizada com commit antes do retorno do erro na camada de serviço.
