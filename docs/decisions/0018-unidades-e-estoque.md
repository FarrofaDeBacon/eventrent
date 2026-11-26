# ADR-0018: Gestão de Unidades Físicas, Estoque Multidepósito e Rastreabilidade

## Status

Aceito (Fase 2B - Unidades Físicas e Estoque)

## Contexto

Até a Fase 2A, o catálogo do EventRent modelava exclusivamente especificações abstratas e fichas técnicas de equipamentos (`equipment_model`, `category`, `brand`, etc.). No modelo de negócios de locação de eventos, no entanto, a operação cotidiana requer o controle rigoroso dos bens materiais físicos que residem nos depósitos, são transportados em cases para os eventos, sofrem avarias, passam por manutenção preventiva/corretiva e retornam ao galpão.

Empresas de locação operam com dois perfis distintos de inventário:

1. **Equipamentos de Alto Valor / Identificação Individual (Serializados)**: Moving lights, consoles de áudio digital, caixas de som ativas, projetores, telões LED e microfones sem fio. Cada exemplar físico possui custo elevado, histórico de manutenção, número de série de fábrica e etiqueta de tombamento/patrimônio.
2. **Itens de Alto Volume / Granel (Quantitativos)**: Cabos balanceados XLR de 10m, extensões de força, garras de fixação (clamp C), adaptadores de energia, fitas de demarcação e ferragens de montagem. O controle individual por número de série é inviável operacionalmente, demandando controle numérico por saldo e ponto de reposição.

Adicionalmente, as locadoras frequentemente possuem múltiplos pontos de estocagem (Galpão Principal, Depósito Secundário, Veículo de Trânsito / Caminhão 1), demandando controle multidepósito com pelo menos um depósito padrão obrigatório.

Para fundamentar as fases subsequentes — em especial o Motor de Disponibilidade e Alocação (ADR-0012) e as Ordens de Serviço / Checklist (Fase 3) —, é mandatário estabelecer uma arquitetura relacional e de domínio que garanta:

- Rastreabilidade inalterável de todas as movimentações físicas (livro-razão append-only).
- Prevenção categórica de estoque negativo por meio de constraints e triggers no banco de dados.
- Tombamento automatizado sequencial com prefixo padronizado (`EQ000001`).
- Segregação de privilégios via RBAC e preservação da privacidade de custos financeiros (`acquisition_cost`, `replacement_cost`) para operadores operacionais sem privilégio financeiro.

---

## Decisões

### 1. Modos de Controle de Estoque (`control_mode`)

Cada modelo de equipamento (`equipment_model`) define seu modo de controle físico através da coluna `control_mode`:

- **`serialized` (Serializado)**:
  - Cada exemplar físico é registrado como uma entidade distinta na tabela `equipment_unit`.
  - Possui patrimônio exclusivo gerado pelo sistema (`patrimony_tag`), número de série do fabricante opcional (`serial_number`) e status operacional individual.
  - O ciclo de vida do item transita entre os seguintes estados canônicos:
    - `available`: Pronto para locação no depósito.
    - `allocated`: Reservado/alocado para ordem de serviço ativa.
    - `in_transit`: Em trânsito logístico de entrega ou retorno.
    - `maintenance`: Em bancada ou assistência técnica externa.
    - `damaged`: Com avaria registrada aguardando perícia ou reparo.
    - `lost`: Dado como extraviado ou furtado.
    - `retired`: Baixado do patrimônio da empresa por obsolescência ou descarte.
- **`bulk` (Quantitativo / A Granel)**:
  - Não gera registros individuais na tabela `equipment_unit`.
  - O controle ocorre exclusivamente por contagem numérica agregada por depósito na tabela `stock_balance`.

---

### 2. Geração Atômica de Patrimônio Sequencial (`equipment_patrimony_seq`)

Para eliminar o risco de etiquetas duplicadas ou furos decorrentes de falhas de concorrência, o tombamento patrimonial de unidades serializadas utiliza uma tabela de controle de sequência atômica por tenant:

- **Formato Canônico**: Prefixo literal `EQ` seguido de 6 dígitos decimais formatados com zeros à esquerda (ex: `EQ000001`, `EQ000002`, ..., `EQ999999`).
- **Tabela `equipment_patrimony_seq`**:
  - Chave primária: `tenant_id`.
  - Coluna: `last_number INTEGER NOT NULL DEFAULT 0`.
- **Garantia Transacional**:
  - A geração do código ocorre na mesma transação SQLite da criação da unidade via `UPDATE equipment_patrimony_seq SET last_number = last_number + 1 WHERE tenant_id = ? RETURNING last_number`.
  - Índice único parcial garante que não existam unidades ativas com o mesmo código de patrimônio dentro de um mesmo tenant: `CREATE UNIQUE INDEX idx_equipment_unit_tenant_patrimony ON equipment_unit(tenant_id, patrimony_tag)`.

---

### 3. Gestão Multidepósito (`warehouse`)

A estrutura física de armazenamento é modelada na tabela `warehouse`:

- **Atributos**: `id`, `tenant_id`, `name`, `code`, `description`, `address_line1`, `address_line2`, `city`, `state`, `postal_code`, `is_default`, `is_active`, `created_at`, `updated_at`.
- **Regra do Depósito Padrão (`is_default`)**:
  - Cada tenant DEVE possuir exatamente **um** depósito padrão.
  - Ao inicializar o tenant (via rotina idempotente `ensure_default_inventory`), o depósito `Depósito principal` (código `PRINCIPAL`) é provisionado com `is_default = 1`. A migração SQL permanece puramente estrutural (DDL).
  - Ao marcar um depósito como `is_default = 1`, o sistema remove automaticamente a flag de outros depósitos do mesmo tenant dentro de transação atômica.
  - Um índice único parcial SQLite reforça no banco que não pode haver mais de um depósito padrão ativo: `CREATE UNIQUE INDEX idx_warehouse_single_default ON warehouse(tenant_id) WHERE is_default = 1`.
- **Integridade Referencial na Exclusão**:
  - Depósitos não podem ser removidos fisicamente se houver saldo ativo (`quantity_on_hand > 0`) em `stock_balance` ou unidades físicas associadas em `equipment_unit`. Nesses casos, apenas a inativação lógica (`is_active = 0`) é permitida, impedindo novas entradas.

---

### 4. Livro-Razão Imutável (`stock_movement`) e Saldo (`stock_balance`)

Para assegurar auditoria fiscal e conformidade contábil, o EventRent adota a arquitetura de **Livro-Razão Imutável (Append-Only Journal)**:

#### A) Tabela `stock_movement`

- Registra cada evento discreto que altera o inventário.
- **Colunas**:
  - `id`: UUID v7 primário.
  - `tenant_id`: Isolamento multi-inquilino.
  - `equipment_model_id`: Modelo movimentado.
  - `equipment_unit_id`: Unidade física específica (obrigatório se modelo for serializado; `NULL` se for bulk).
  - `from_warehouse_id`: Depósito de origem (`NULL` para compras e inventários iniciais).
  - `to_warehouse_id`: Depósito de destino (`NULL` para descartes, baixas ou perdas).
  - `movement_type`: Tipo canônico de operação:
    - `purchase_in`: Entrada por aquisição de novos itens.
    - `return_in`: Retorno de locação ou evento.
    - `rental_out`: Saída para evento / ordem de locação.
    - `transfer_out`: Saída por transferência interdepósitos.
    - `transfer_in`: Entrada por transferência interdepósitos.
    - `maintenance_out`: Envio de equipamento para manutenção.
    - `maintenance_in`: Retorno de bancada / conserto.
    - `loss_out`: Baixa por extravio, roubo ou dano irreparável.
    - `adjustment`: Ajuste físico decorrente de contagem de inventário.
  - `quantity`: Quantidade movimentada (inteiro positivo `> 0`).
  - `reason`: Justificativa obrigatória do movimento.
  - `performed_by_user_id`: Operador responsável pela ação.
  - `created_at`: Carimbo de data/hora UTC.
- **Triggers Anti-Adulteração**:
  - Triggers SQLite bloqueiam categoricamente qualquer operação de `UPDATE` ou `DELETE` na tabela `stock_movement`:
    ```sql
    CREATE TRIGGER trg_stock_movement_no_update
    BEFORE UPDATE ON stock_movement
    BEGIN
        SELECT RAISE(ABORT, 'Operação proibida: stock_movement é imutável (append-only)');
    END;

    CREATE TRIGGER trg_stock_movement_no_delete
    BEFORE DELETE ON stock_movement
    BEGIN
        SELECT RAISE(ABORT, 'Operação proibida: registros de stock_movement não podem ser excluídos');
    END;
    ```

#### B) Tabela `stock_balance`

- Cache agregado persistente de saldos por depósito e modelo.
- **Colunas**: `id`, `tenant_id`, `warehouse_id`, `equipment_model_id`, `quantity_on_hand`, `quantity_allocated`, `quantity_maintenance`, `updated_at`.
- **Chave Única**: `UNIQUE(tenant_id, warehouse_id, equipment_model_id)`.
- **Integridade de Saldo Não-Negativo**:
  - `CHECK (quantity_on_hand >= 0 AND quantity_allocated >= 0 AND quantity_maintenance >= 0)`.
  - Triggers de consistência garantem que movimentações que resultariam em saldo negativo abortem imediatamente a transação com erro de negócio explícito.

---

### 5. Anexos de Mídia por Unidade (`equipment_unit_media`)

Para subsidiar perícias de devolução, termos de vistoria e histórico de sinistros, unidades serializadas suportam fotos e documentos dedicados:

- Tabela de junção `equipment_unit_media` associando `equipment_unit_id` ao `media_blob_id` (arquitetura CAS SHA-256 do ADR-0013).
- Tipos de anexo: `damage_photo` (foto de avaria), `inspection_photo` (laudo de vistoria), `serial_photo` (foto da placa de número de série), `invoice_doc` (nota fiscal de compra em PDF).

---

### 6. Isolamento Transacional e Concorrência

Todas as mutações de inventário (criação de unidades, transferências entre depósitos, envios para manutenção e baixas) são encapsuladas em transações atômicas gerenciadas pelo `TransactionManager` do Core:

- Em modo WAL, leituras concorrentes nunca bloqueiam escritas e escritas nunca bloqueiam leituras.
- Concorrência de escritas na mesma linha de `stock_balance` é protegida com retry automático e backoff exponencial em caso de `SQLITE_BUSY`.

---

### 7. RBAC e Segregação de Privacidade Financeira

O catálogo e estoque implementam políticas rígidas de menor privilégio:

1. **Permissões Canônicas do Módulo**:
   - `inventory:read`: Permite consultar depósitos, unidades físicas, saldos e histórico de movimentações.
   - `inventory:manage`: Permite cadastrar e editar depósitos, cadastrar unidades físicas, registrar avarias, alterar status e lançar movimentações de estoque.
2. **Mascaramento de Confidencialidade Financeira**:
   - Os modelos de equipamento possuem campos de valoração: `acquisition_cost` (custo de aquisição) e `replacement_cost` (custo de reposição).
   - Na camada do Rust Core (`eventrent-core`), ao serializar modelos ou unidades para transporte IPC, o serviço verifica se o usuário autenticado na sessão possui a permissão `finance:read`.
   - Se o usuário possuir apenas `inventory:read` (como técnicos e operadores de depósito), os campos `acquisition_cost` e `replacement_cost` são sanitizados para `None` (`null` no JSON / IPC).
   - O schema Zod no frontend valida esses campos como opcionais (`z.number().nullable().optional()`), impedindo vazamento de dados sensíveis da locadora para colaboradores sem autorização financeira.

---

### 8. Esquema Relacional Formal (Migração `0011_inventory.sql`)

A migração `0011_inventory.sql` é a única autorizada para expansão do banco de dados na Fase 2B, preservando congeladas as migrações `0001` a `0010`:

```sql
-- 1. Depósitos (warehouses)
CREATE TABLE warehouse (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    name TEXT NOT NULL,
    code TEXT NOT NULL,
    description TEXT,
    address_line1 TEXT,
    address_line2 TEXT,
    city TEXT,
    state TEXT,
    postal_code TEXT,
    is_default INTEGER NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX idx_warehouse_tenant_code ON warehouse(tenant_id, code);
CREATE UNIQUE INDEX idx_warehouse_single_default ON warehouse(tenant_id) WHERE is_default = 1;

-- 2. Sequência de Patrimônio por Tenant
CREATE TABLE equipment_patrimony_seq (
    tenant_id TEXT PRIMARY KEY,
    last_number INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE
);

-- 3. Unidades Físicas Serializadas (equipment_unit)
CREATE TABLE equipment_unit (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    equipment_model_id TEXT NOT NULL,
    warehouse_id TEXT NOT NULL,
    patrimony_tag TEXT NOT NULL,
    serial_number TEXT,
    status TEXT NOT NULL DEFAULT 'available',
    physical_condition TEXT NOT NULL DEFAULT 'good',
    notes TEXT,
    acquisition_date TEXT,
    warranty_expires_at TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (equipment_model_id) REFERENCES equipment_model(id) ON DELETE RESTRICT,
    FOREIGN KEY (warehouse_id) REFERENCES warehouse(id) ON DELETE RESTRICT
);

CREATE UNIQUE INDEX idx_equipment_unit_tenant_patrimony ON equipment_unit(tenant_id, patrimony_tag);
CREATE INDEX idx_equipment_unit_model_status ON equipment_unit(tenant_id, equipment_model_id, status);
CREATE INDEX idx_equipment_unit_warehouse ON equipment_unit(tenant_id, warehouse_id);

-- 4. Saldo Agregado por Depósito (stock_balance)
CREATE TABLE stock_balance (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    warehouse_id TEXT NOT NULL,
    equipment_model_id TEXT NOT NULL,
    quantity_on_hand INTEGER NOT NULL DEFAULT 0,
    quantity_allocated INTEGER NOT NULL DEFAULT 0,
    quantity_maintenance INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (warehouse_id) REFERENCES warehouse(id) ON DELETE RESTRICT,
    FOREIGN KEY (equipment_model_id) REFERENCES equipment_model(id) ON DELETE RESTRICT,
    CONSTRAINT chk_stock_balance_positive CHECK (
        quantity_on_hand >= 0 AND
        quantity_allocated >= 0 AND
        quantity_maintenance >= 0
    )
);

CREATE UNIQUE INDEX idx_stock_balance_unique ON stock_balance(tenant_id, warehouse_id, equipment_model_id);

-- 5. Livro-Razão Imutável de Movimentações (stock_movement)
CREATE TABLE stock_movement (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    equipment_model_id TEXT NOT NULL,
    equipment_unit_id TEXT,
    from_warehouse_id TEXT,
    to_warehouse_id TEXT,
    movement_type TEXT NOT NULL,
    quantity INTEGER NOT NULL,
    reason TEXT NOT NULL,
    reference_id TEXT,
    performed_by_user_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (equipment_model_id) REFERENCES equipment_model(id) ON DELETE RESTRICT,
    FOREIGN KEY (equipment_unit_id) REFERENCES equipment_unit(id) ON DELETE SET NULL,
    FOREIGN KEY (from_warehouse_id) REFERENCES warehouse(id) ON DELETE RESTRICT,
    FOREIGN KEY (to_warehouse_id) REFERENCES warehouse(id) ON DELETE RESTRICT,
    FOREIGN KEY (performed_by_user_id) REFERENCES user(id) ON DELETE RESTRICT,
    CONSTRAINT chk_stock_movement_qty CHECK (quantity > 0)
);

CREATE INDEX idx_stock_movement_tenant_model ON stock_movement(tenant_id, equipment_model_id, created_at);
CREATE INDEX idx_stock_movement_unit ON stock_movement(tenant_id, equipment_unit_id);

-- 6. Mídia Vinculada à Unidade Física (equipment_unit_media)
CREATE TABLE equipment_unit_media (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    equipment_unit_id TEXT NOT NULL,
    media_blob_id TEXT NOT NULL,
    media_type TEXT NOT NULL,
    caption TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (equipment_unit_id) REFERENCES equipment_unit(id) ON DELETE CASCADE,
    FOREIGN KEY (media_blob_id) REFERENCES media_blob(id) ON DELETE RESTRICT
);

CREATE INDEX idx_equipment_unit_media_unit ON equipment_unit_media(tenant_id, equipment_unit_id);
```

---

## Consequências

### Positivas

- **Auditoria Total**: Nenhuma peça de equipamento ou item de estoque entra, sai ou é avariado sem um registro auditável em `stock_movement`.
- **Imutabilidade Real**: Triggers de banco protegem o histórico contra modificações maliciosas ou acidentais, garantindo conformidade jurídica e fiscal.
- **Base Sólida para o Motor de Disponibilidade**: Permite consultas determinísticas de saldo real por depósito e período de locação (ADR-0012).
- **Sigilo Comercial**: Proteção automática de dados financeiros contra usuários operacionais sem privilégio `finance:read`.

### Desafios e Mitigações

- **Custos de Concorrência**: Atualizações frequentes de `stock_balance` em depósitos com movimentações intensas podem gerar contenção transitória. Mitigado pela brevidade das transações no SQLite em modo WAL e gerenciador de retries com backoff.
- **Risco de Estoque Fantasma**: Eliminado pela constraint de saldo não-negativo (`CHECK (quantity_on_hand >= 0)`) e integridade referencial estrita.
