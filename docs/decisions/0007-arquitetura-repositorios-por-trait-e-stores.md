# ADR-0007: Arquitetura de Repositórios por Trait (Ports) e Separação de Crates de Store

## Contexto

Embora instruções SQL básicas possam ser escritas de forma semelhante entre SQLite e PostgreSQL, o `sqlx` utiliza marcadores de parâmetros posicionais distintos (`?` no SQLite versus `$1, $2` no PostgreSQL) e tipos de pool específicos (`Pool<Sqlite>` vs `Pool<Postgres>`). Manter `sqlx` diretamente dentro de `crates/core` acoplaria o núcleo de domínio a um único banco de dados.

## Decisão

1. **`crates/core` Independente de Banco de Dados**:
   - `crates/core` não possui dependência de `sqlx`.
   - Contém apenas entidades de domínio, regras de negócio, serviços (`CoreServices`), criptografia (`argon2`, `rand`) e **traits assíncronas ("ports")** de persistência: `TenantStore`, `BrandingStore`, `RoleStore`, `UserStore` e `AuditStore`.
2. **Crates Dedicados por Banco de Dados**:
   - `crates/store-sqlite`: Implementa todas as traits do `core` para SQLite e concentra as migrações SQLite (`crates/store-sqlite/migrations`).
   - `crates/store-postgres` (previsto para a Fase 2.5): Implementará as mesmas traits do `core` para PostgreSQL no servidor `axum`.
3. **Suíte Genérica de Testes de Contrato**:
   - `crates/core::testing::run_store_contract_suite` define uma suíte de testes agnóstica que recebe uma fábrica assíncrona de `StoreBundle` e valida todas as traits e regras de serviço.
   - Hoje a suíte é executada por `crates/store-sqlite` em memória; na Fase 2.5 será executada também por `crates/store-postgres`.

## Consequências

- **Positivas**: Desacoplamento total entre regras de negócio e motor de banco de dados; zero retrabalho nos serviços do `core` ao adicionar PostgreSQL na Fase 2.5.
- **Negativas/Atenção**: Novas operações de persistência exigem declaração do método na trait em `crates/core` e implementação no crate de store correspondente.
