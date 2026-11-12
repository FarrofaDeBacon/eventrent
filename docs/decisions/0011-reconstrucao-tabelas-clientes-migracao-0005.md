# ADR-0011: Reconstrução das Tabelas de Clientes na Migração 0005

- **Status**: Aceito
- **Data**: 2026-11-12
- **Fase**: 1B.2-rev

## Contexto

A migração `0004_customers.sql` foi incorporada à branch `main` no merge do PR #6 e, portanto, passou a integrar o conjunto de migrações estritamente congeladas (`scripts/check-migrations.js`). Durante a revisão técnica da Fase 1B.2, foram identificadas lacunas estruturais no esquema inicial de clientes:

1. `customer_contact` e `customer_address` não possuíam `tenant_id` (obrigatório para isolamento multi-tenant e futura política RLS no PostgreSQL) nem `created_by`.
2. Faltava o campo `country` em `customer_address` e o índice único parcial garantindo no máximo um contato/endereço principal (`WHERE is_primary = 1 AND deleted_at IS NULL`).
3. `customer.document_number` era `NOT NULL` e não havia coluna `document_type` (`cpf`, `cnpj`, `foreign`, `none`), impedindo pré-cadastro sem documento e cadastro de clientes estrangeiros.
4. Faltava a coluna `active` em `customer` para ativação/desativação (`set_active`).
5. O índice `idx_customer_tenant_search` era peso morto nas gravações, pois consultas de trecho (`LIKE '%termo%'`) realizam varredura filtrada por `tenant_id`.

Como o SQLite não suporta alterações completas de restrições `CHECK`, nulabilidade (`DROP NOT NULL`) e `FOREIGN KEY` via `ALTER TABLE`, torna-se necessária a reconstrução controlada das tabelas seguindo o procedimento oficial de 12 passos da documentação do SQLite.

## Decisão

1. **Exceção Única na Migração `0005_customers_rev.sql`**: Reconstruir as tabelas `customer`, `customer_contact` e `customer_address` preservando integralmente eventuais dados existentes (tabelas auxiliares de cópia -> `DROP` em ordem filha-para-pai -> `CREATE` -> `INSERT` -> `DROP` das tabelas auxiliares -> criação dos novos índices).
2. **Justificativa da Exceção**: Nenhuma versão distribuída aos usuários finais contém o módulo de clientes ainda. Esta reconstrução evita carregar dívida estrutural para a Fase 2 e para a migração PostgreSQL da Fase 2.5.
3. **Imutabilidade da `0004`**: A migração `0004_customers.sql` permanece intocada na `main`.

## Consequências

- Isolamento multi-tenant completo (`tenant_id NOT NULL` com FK) em todas as tabelas do agregado de clientes.
- Unicidade parcial por `(tenant_id, document_type, document_number) WHERE document_number IS NOT NULL AND deleted_at IS NULL`, permitindo múltiplos pré-cadastros sem documento.
- Eliminação do índice morto `idx_customer_tenant_search`.
