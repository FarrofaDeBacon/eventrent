# ADR-0018: Gestão de Unidades Físicas, Estoque Multidepósito e Rastreabilidade

## Status

Aceito (Fase 2B-rev2 - Unidades Físicas e Estoque)

## Contexto

Até a Fase 2A, o catálogo do EventRent modelava exclusivamente especificações abstratas e fichas técnicas de equipamentos (`equipment_model`, `category`, `brand`, etc.). No modelo de negócios de locação de eventos, no entanto, a operação cotidiana requer o controle rigoroso dos bens materiais físicos que residem nos depósitos, são transportados em cases para os eventos, sofrem avarias, passam por manutenção preventiva/corretiva e retornam ao galpão.

Empresas de locação operam com dois perfis distintos de inventário:

1. **Equipamentos de Alto Valor / Identificação Individual (Serializados)**: Moving lights, consoles de áudio digital, caixas de som ativas, projetores, telões LED e microfones sem fio. Cada exemplar físico possui custo elevado, histórico de manutenção, número de série de fábrica e etiqueta de tombamento/patrimônio.
2. **Itens de Alto Volume / Granel (Quantitativos)**: Cabos balanceados XLR de 10m, extensões de força, garras de fixação (clamp C), adaptadores de energia, fitas de demarcação e ferragens de montagem. O controle individual por número de série é inviável operacionalmente, demandando controle numérico por saldo e ponto de reposição (`min_quantity`).

Adicionalmente, as locadoras frequentemente possuem múltiplos pontos de estocagem (Galpão Principal, Depósito Secundário, Veículo de Trânsito / Caminhão 1), demandando controle multidepósito com pelo menos um depósito padrão obrigatório.

Para fundamentar as fases subsequentes — em especial o Motor de Disponibilidade e Alocação (ADR-0012) e as Ordens de Serviço / Checklist (Fase 3 e Fase 5) —, é mandatário estabelecer uma arquitetura relacional e de domínio que garanta:

- Rastreabilidade inalterável de todas as movimentações físicas (livro-razão append-only).
- Prevenção categórica de estoque negativo por meio de constraints e triggers no banco de dados.
- Tombamento automatizado sequencial com prefixo padronizado (`EQ000001`).
- Segregação estrita de privilégios via RBAC com as permissões dedicadas `inventory:read` e `inventory:manage` (reservando `warehouse:read` e `warehouse:operate` para conferência de carga na Fase 5).
- Redação centralizada de custos financeiros (`acquisition_cost_cents`, `replacement_value_cents`, `cost_price_cents`, dados de nota e fornecedor) para usuários sem o privilégio `finance:read`.
- Rejeição explícita com `Forbidden` para qualquer tentativa de mutação financeira sem `finance:manage`.
- Controle otimista de concorrência com `expected_updated_at` em todas as mutações dependentes de estado.

---

## Decisões

### 1. Modos de Controle de Estoque (`control_mode`)

Cada modelo de equipamento (`equipment_model`) define seu modo de controle físico através da coluna `control_mode`:

- **`serialized` (Serializado)**:
  - Cada exemplar físico é registrado como uma entidade distinta na tabela `equipment_unit`.
  - Possui patrimônio exclusivo gerado pelo sistema (`patrimony_tag`), número de série do fabricante opcional (`serial_number`) e status operacional individual.
  - O ciclo de vida do item transita exclusivamente entre os seguintes estados canônicos (evitando fontes duplicadas da verdade):
    - `available`: Disponível e operacional no depósito.
    - `maintenance`: Em bancada ou assistência técnica externa.
    - `damaged`: Com avaria registrada aguardando perícia ou reparo.
    - `lost`: Dado como extraviado ou furtado (transição para `available` exige justificativa obrigatória).
    - `decommissioned`: Baixado definitivamente do patrimônio da empresa (estado terminal irreversível).
  - _Nota Arquitetural_: Os status `allocated` e `in_transit`, bem como a coluna `quantity_allocated`, foram removidos da persistência física de unidade e saldo para evitar descompasso com duas fontes da verdade; tais estados serão derivados dinamicamente das alocações na Fase 3.
- **`bulk` (Quantitativo / A Granel)**:
  - Não gera registros individuais na tabela `equipment_unit`.
  - O controle ocorre exclusivamente por contagem numérica agregada por depósito na tabela `stock_balance`.
  - A tentativa de cadastrar unidade em modelo `bulk` é rejeitada com erro de validação.
  - A alteração de `control_mode` de um modelo é sumariamente bloqueada se já existirem unidades, movimentações ou saldo associados.

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

- **Atributos**: `id`, `tenant_id`, `name`, `code`, `description`, `address_line1`, `address_line2`, `city`, `state`, `postal_code`, `is_default`, `active`, `created_at`, `updated_at`, `deleted_at`, `created_by`.
- **Regra do Depósito Padrão (`is_default`)**:
  - Cada tenant DEVE possuir exatamente **um** depósito padrão.
  - Ao inicializar o tenant (via rotina idempotente `ensure_default_inventory`), o depósito `Depósito principal` (código `PRINCIPAL`) é provisionado com `is_default = 1`. A migração SQL permanece puramente estrutural (DDL).
  - Ao marcar um depósito como `is_default = 1`, o sistema remove automaticamente a flag de outros depósitos do mesmo tenant dentro de transação atômica.
  - Um índice único parcial SQLite reforça no banco que não pode haver mais de um depósito padrão ativo: `CREATE UNIQUE INDEX idx_warehouse_single_default ON warehouse(tenant_id) WHERE is_default = 1 AND deleted_at IS NULL`.
- **Integridade Referencial na Exclusão**:
  - Depósitos não podem ser removidos fisicamente se houver saldo ativo (`quantity_on_hand > 0`) em `stock_balance` ou unidades físicas associadas em `equipment_unit`. Nesses casos, apenas a inativação ou exclusão lógica é permitida, impedindo novas entradas.

---

### 4. Livro-Razão Imutável (`stock_movement`) e Saldo (`stock_balance`)

Para assegurar auditoria fiscal e conformidade contábil, o EventRent adota a arquitetura de **Livro-Razão Imutável (Append-Only Journal)**:

#### A) Tabela `stock_movement`

- Registra cada evento discreto que altera o inventário.
- **Colunas**:
  - `id`: UUID v7 primário.
  - `tenant_id`: Isolamento multi-inquilino.
  - `equipment_model_id`: Modelo movimentado.
  - `equipment_unit_id`: Unidade física específica (`NULL` se for modelo quantitativo).
  - `from_warehouse_id`: Depósito de origem (`NULL` para compras e inventários iniciais).
  - `to_warehouse_id`: Depósito de destino (`NULL` para descartes, baixas ou perdas).
  - `movement_type`: Tipo canônico de operação:
    - `purchase_in`: Entrada por aquisição de novos itens.
    - `return_in`: Retorno de locação ou devolução.
    - `transfer_out`: Saída por transferência interdepósitos.
    - `transfer_in`: Entrada por transferência interdepósitos.
    - `status_change`: Alteração de status físico (manutenção/avaria) sem alterar o saldo físico global do depósito (`quantity_on_hand`).
    - `loss_out`: Saída de estoque por perda ou extravio.
    - `decommission_out`: Saída definitiva de estoque por baixa patrimonial.
    - `adjustment`: Ajuste físico decorrente de contagem de inventário.
  - `quantity`: Quantidade movimentada (inteiro positivo `> 0`).
  - `reason`: Justificativa obrigatória do movimento.
  - `reference_id`: Identificador de documento ou lote externo.
  - `created_by`: Usuário operador.
  - `created_at`: Carimbo de data/hora UTC.
- **Regra de Movimentos Concomitantes**:
  - Quando uma unidade física sofre alteração de depósito **e** de status na mesma operação, o sistema grava **dois** lançamentos atômicos distintos no livro-razão dentro da mesma transação (um de transferência e um de mudança de status).
  - Registro de avaria (`damaged`) ou manutenção (`maintenance`) NÃO é considerado saída de estoque, utilizando `status_change`. Saídas do saldo físico (`loss_out`, `decommission_out`) aplicam-se apenas a extravio ou descarte definitivo.
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
- **Colunas**: `id`, `tenant_id`, `warehouse_id`, `equipment_model_id`, `quantity_on_hand`, `quantity_maintenance`, `min_quantity`, `updated_at`.
- **Chave Única**: `UNIQUE(tenant_id, warehouse_id, equipment_model_id)`.
- **Integridade de Saldo Não-Negativo**:
  - `CHECK (quantity_on_hand >= 0 AND quantity_maintenance >= 0 AND min_quantity >= 0)`.
  - Triggers de consistência garantem que movimentações que resultariam em saldo negativo abortem imediatamente a transação com erro de negócio explícito.
  - `min_quantity` permite configurar o ponto de reposição/alerta de estoque mínimo.

---

### 5. Baixa Patrimonial (`decommission`) vs Exclusão (`delete`)

- **Baixa Patrimonial (`decommission_equipment_unit`)**:
  - É o ciclo de vida normal e formal de encerramento de um equipamento.
  - Exige justificativa obrigatória, `expected_updated_at` para controle de concorrência, altera o status para `decommissioned`, inativa a unidade (`active = 0`), registra evento na auditoria e lança movimento `decommission_out` no livro-razão debitando o estoque.
  - O status `decommissioned` é terminal e irreversível.
- **Exclusão Lógica (`delete_equipment_unit`)**:
  - É restrita exclusivamente a unidades **criadas por engano**.
  - Só é permitida se a unidade tiver sido criada há **menos de 24 horas** E não possuir movimentações posteriores além do tombamento inicial de compra (`<= 1` registro no livro-razão).
  - Tentativas de exclusão fora dessa janela retornam `CoreError::Conflict`, orientando o operador a realizar a baixa formal. Na Fase 3, incluirá checagem de alocações futuras.

---

### 6. RBAC e Segregação de Privacidade Financeira (R1 / R5)

1. **Permissões Canônicas do Módulo**:
   - `inventory:read`: Permite consultar depósitos, unidades físicas, saldos e histórico de movimentações (concedida a Proprietário, Estoque, Comercial, Técnico e Financeiro).
   - `inventory:manage`: Permite cadastrar e editar depósitos, cadastrar unidades físicas, alterar status, baixar patrimônio e lançar movimentações no livro-razão (concedida a Proprietário e Estoque; negada a Técnico, Comercial e Financeiro).
   - As permissões `warehouse:read` e `warehouse:operate` continuam reservadas para os fluxos de conferência física de carga na Fase 5.
2. **Redação Financeira Centralizada**:
   - Campos confidenciais em `equipment_unit`: `acquisition_cost_cents`, `supplier_id` (FK para `customer`), `invoice_number`.
   - Campos confidenciais em `equipment_model`: `cost_price_cents`, `replacement_value_cents`.
   - Se a sessão do usuário não possuir privilégio `finance:read` (ou não for Proprietário), todos esses campos são mascarados para `None` em todas as rotas (unidades, listas, busca, movimentos e modelos).
   - A serialização JSON omite campos confidenciais via `#[serde(skip_serializing_if = "Option::is_none")]`.
   - Tentativa de mutação desses campos sem `finance:manage` retorna erro explícito `CoreError::Forbidden`.
   - O registro de auditoria omite valores monetários confidenciais.

---

### 7. Otimização de Performance e Mídia de Unidade (R6 / R7)

1. **Consulta Otimizada sem N+1**:
   - `list_equipment_units` executa consulta única via `INNER JOIN equipment_model` e `INNER JOIN warehouse`, eliminando overhead de N+1 consultas.
   - Paginação por cursor padronizada (padrão 50, máx 100 itens) e busca textual limitada a 100 caracteres e 8 termos.
2. **Adiantamento da Mídia de Unidade Física**:
   - A tabela `equipment_unit_media` e os comandos IPC inacabados foram retirados da migração `0011_inventory.sql`.
   - O suporte completo a fotos e vistorias de unidades físicas foi reprogramado para a Fase 2C, onde integrará diretamente com a arquitetura `media_asset` da Fase 2M (com miniaturas WebP, títulos, créditos e integridade referencial completa).

---

### 8. Esquema Relacional Formal (Migração `0011_inventory.sql`)

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
    active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES app_user(id) ON DELETE SET NULL
);

CREATE UNIQUE INDEX idx_warehouse_tenant_code ON warehouse(tenant_id, code) WHERE deleted_at IS NULL;
CREATE UNIQUE INDEX idx_warehouse_single_default ON warehouse(tenant_id) WHERE is_default = 1 AND deleted_at IS NULL;

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
    acquisition_cost_cents INTEGER,
    supplier_id TEXT,
    invoice_number TEXT,
    origin TEXT NOT NULL DEFAULT 'own',
    return_by TEXT,
    usage_hours INTEGER NOT NULL DEFAULT 0,
    location_note TEXT,
    active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    created_by TEXT,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (equipment_model_id) REFERENCES equipment_model(id) ON DELETE RESTRICT,
    FOREIGN KEY (warehouse_id) REFERENCES warehouse(id) ON DELETE RESTRICT,
    FOREIGN KEY (tenant_id, supplier_id) REFERENCES customer(tenant_id, id) ON DELETE SET NULL,
    FOREIGN KEY (created_by) REFERENCES app_user(id) ON DELETE SET NULL,
    CONSTRAINT chk_unit_status CHECK (
        status IN ('available', 'maintenance', 'damaged', 'lost', 'decommissioned')
    ),
    CONSTRAINT chk_unit_condition CHECK (
        physical_condition IN ('new', 'good', 'fair', 'poor')
    ),
    CONSTRAINT chk_unit_origin CHECK (
        origin IN ('own', 'third_party')
    ),
    CONSTRAINT chk_unit_cost_positive CHECK (
        acquisition_cost_cents IS NULL OR acquisition_cost_cents >= 0
    )
);

CREATE UNIQUE INDEX idx_equipment_unit_tenant_patrimony ON equipment_unit(tenant_id, patrimony_tag) WHERE deleted_at IS NULL;
CREATE INDEX idx_equipment_unit_model_status ON equipment_unit(tenant_id, equipment_model_id, status) WHERE deleted_at IS NULL;
CREATE INDEX idx_equipment_unit_warehouse ON equipment_unit(tenant_id, warehouse_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_equipment_unit_search ON equipment_unit(tenant_id, serial_number, patrimony_tag);

-- 4. Saldo Agregado por Depósito (stock_balance)
CREATE TABLE stock_balance (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    warehouse_id TEXT NOT NULL,
    equipment_model_id TEXT NOT NULL,
    quantity_on_hand INTEGER NOT NULL DEFAULT 0,
    quantity_maintenance INTEGER NOT NULL DEFAULT 0,
    min_quantity INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (warehouse_id) REFERENCES warehouse(id) ON DELETE RESTRICT,
    FOREIGN KEY (equipment_model_id) REFERENCES equipment_model(id) ON DELETE RESTRICT,
    CONSTRAINT chk_stock_balance_positive CHECK (
        quantity_on_hand >= 0 AND
        quantity_maintenance >= 0 AND
        min_quantity >= 0
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
    created_by TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    FOREIGN KEY (equipment_model_id) REFERENCES equipment_model(id) ON DELETE RESTRICT,
    FOREIGN KEY (equipment_unit_id) REFERENCES equipment_unit(id) ON DELETE SET NULL,
    FOREIGN KEY (from_warehouse_id) REFERENCES warehouse(id) ON DELETE RESTRICT,
    FOREIGN KEY (to_warehouse_id) REFERENCES warehouse(id) ON DELETE RESTRICT,
    FOREIGN KEY (created_by) REFERENCES app_user(id) ON DELETE SET NULL,
    CONSTRAINT chk_stock_movement_qty CHECK (quantity > 0),
    CONSTRAINT chk_stock_movement_type CHECK (
        movement_type IN (
            'purchase_in', 'return_in', 'transfer_out', 'transfer_in',
            'status_change', 'loss_out', 'decommission_out', 'adjustment'
        )
    )
);

CREATE INDEX idx_stock_movement_tenant_model ON stock_movement(tenant_id, equipment_model_id, created_at);
CREATE INDEX idx_stock_movement_unit ON stock_movement(tenant_id, equipment_unit_id);
```

---

## Consequências

### Positivas

- **Auditoria Total e Imutabilidade Real**: Triggers de banco e constraints impedem alterações ou exclusões no livro-razão `stock_movement`.
- **Consistência de Estado Único**: Remoção de estados deriváveis evita divergência com a futura Fase 3 de Alocações.
- **Segurança e Privacidade Financeira**: Operadores sem `finance:read` nunca acessam custos de compra, reposição, fornecedor ou nota fiscal. Mutação sem `finance:manage` gera erro imediato `Forbidden`.
- **Prevenção Concorrencial**: Controle de versão otimista com `expected_updated_at` em todas as mutações de depósitos e unidades.
