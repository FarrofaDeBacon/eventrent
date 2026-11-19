# Requisitos de Produto — Fases 2 a 4.5

Este documento consolida os requisitos de produto e regras de domínio que governam o desenvolvimento do **EventRent** da Fase 2 até a Fase 4.5. É fonte obrigatória de consulta para todas as decisões de modelagem, serviços, interface e testes.

---

## 1. Catálogo de Equipamentos (Fase 2A)

1. **Hierarquia de Categorias**:
   - Categorias organizadas em árvore com profundidade máxima de **4 níveis** (ex.: `Áudio > Microfones > Sem Fio > Bastão`).
   - Proibição de ciclos na árvore (`parent_id` nunca pode apontar para si mesmo ou para um descendente).
   - Nome único por nível dentro do mesmo `tenant_id` (normalizado sem acentos e em minúsculas), considerando apenas registros ativos (`deleted_at IS NULL`).
   - Exclusão bloqueada caso a categoria possua subcategorias ativas ou modelos de equipamentos ativos vinculados.

2. **Marcas**:
   - Cadastro de marcas único por `tenant_id` (`name_normalized` único entre registros ativos).
   - Suporte à criação rápida (inline) durante o cadastro de modelo de equipamento.
   - Exclusão bloqueada caso existam modelos de equipamentos ativos vinculados à marca.

3. **Campos Técnicos Personalizáveis por Categoria (Herança)**:
   - Cada categoria pode definir até **50 definições de atributos** (`attribute_definition`) com tipos: `text`, `number`, `boolean` e `select` (com lista fechada de opções em `choices_json`).
   - **Herança na árvore**: Um modelo vinculado a uma categoria filha herda automaticamente todos os campos definidos nas categorias ancestrais (da raiz até a categoria do modelo).
   - Valores dos atributos são validados conforme o tipo definido (`number` numérico finito, `boolean` verdadeiro/falso, `select` pertencente às opções configuradas, `is_required` obrigatório).
   - Remoção ou edição de definição de atributo preserva integridade referencial com os valores já armazenados.

4. **Modelos de Equipamento (`equipment_model`)**:
   - Identificação: `name` (obrigatório, até 150 caracteres), `brand_id` (opcional), `category_id` (obrigatório), `model_code` (único por marca dentro do tenant quando preenchido), `sku` (único por tenant quando preenchido), `ean` (código de barras GTIN-8/UPC-12/EAN-13/GTIN-14 com validação de dígito verificador), `ncm` (8 dígitos numéricos), `description` (até 2000 caracteres).
   - **Modo de controle de estoque (`control_mode`)**:
     - `serialized`: controle individual por número de série / etiqueta de patrimônio (ex.: mesas de som, projetores, notebooks, câmeras).
     - `quantity`: controle por saldo/quantidade agrupada (ex.: cabos XLR, adaptadores, extensões, pedestais simples).
   - **Dados físicos e elétricos**:
     - `weight_kg` (peso unitário em kg, positivo quando informado).
     - Dimensões (`length_cm`, `width_cm`, `height_cm` em cm, positivos quando informados).
     - `power_watts` (consumo elétrico em watts, >= 0), `voltage` (`110`, `220` ou `bivolt`), `amperage` (corrente em A, > 0).
     - `requires_case` (flag booleana indicando necessidade de case/flight case), `units_per_case` (inteiro positivo quando aplicável).
   - **Precificação base (sempre em centavos inteiros >= 0)**:
     - `daily_rate_cents` (valor padrão da diária), `replacement_value_cents` (valor de reposição/seguro), `cost_price_cents` (custo de aquisição).
   - **Carga sugerida inicial**:
     - Ação explícita e idempotente para carregar árvore sugerida de categorias Audiovisuais (Áudio, Iluminação, Vídeo/Projeção, Informática, Estrutura/Box Truss, Energia/Cabeamento) com seus campos técnicos iniciais. Nunca executada automaticamente em migração SQL.

---

## 2. Estoque Físico e Kits (Fase 2B)

1. **Unidades Serializadas (`asset` / patrimônio)**:
   - Cada unidade física de modelo `serialized` possui número de série, código de patrimônio/etiqueta (QR/código de barras único por tenant), condição física (`new`, `good`, `fair`, `maintenance`, `retired`), data de aquisição e localização padrão (depósito/prateleira).
   - Transições de status auditadas (`available`, `reserved`, `in_use`, `in_transit`, `maintenance`, `lost`, `retired`).

2. **Saldos por Quantidade (`stock_balance`)**:
   - Para modelos `quantity`, controle de saldo total, saldo em manutenção/quarentena e saldo avariado por depósito/localização, com movimentações auditadas (entrada, ajuste, baixa, transferência).

3. **Kits e Combos**:
   - Kit possui nome, SKU, diária própria (opcional, em centavos) ou soma dos itens, e lista de componentes (`equipment_model_id` + quantidade).
   - Kits **expandem** em seus modelos componentes no momento da checagem de disponibilidade, reserva e geração de romaneio de separação.
   - Prevenção de recursão/ciclos caso kits compostos sejam suportados futuramente.

---

## 3. Portabilidade Multi-Usuário e PostgreSQL (Fase 2.5)

1. **Paridade SQLite ↔ PostgreSQL**:
   - `crates/store-postgres` implementa exatamente as mesmas traits de `crates/core` que `crates/store-sqlite`, validado pela mesma suíte de contrato (`run_store_contract_suite`).
   - Uso de tipos portáveis (`BOOLEAN`, `TIMESTAMPTZ`, inteiros de centavos), placeholders `$n` e concorrência otimista via `UPDATE ... WHERE updated_at = $expected` ou `SELECT ... FOR UPDATE`.
2. **Sessões e Segurança em Rede**:
   - Sessões estritamente server-side (`HttpOnly; Secure; SameSite=Lax`), sem JWT stateless sem revogação.
   - Rate limiting por IP e dispositivo para evitar DoS por lockout de conta.

---

## 4. Motor de Disponibilidade, Reservas e Logística (Fase 3)

1. **Modelo de Três Tempos (ADR-0012)**:
   - Toda reserva ocupa o equipamento considerando:
     - **Janela de uso/evento** (`event_start` .. `event_end`) em um `venue_space` específico.
     - **Tempo de virada / preparação** no mesmo espaço (`allocation.changeover_minutes`, padrão 30 min).
     - **Deslocamento interno** entre espaços do mesmo local (`venue.internal_transfer_minutes` ou padrão `allocation.internal_transfer_minutes`, padrão 20 min).
     - **Deslocamento externo** entre locais diferentes (`allocation.external_transfer_minutes`, padrão 60 min).
   - Dois eventos consecutivos no mesmo espaço respeitam o intervalo de virada; em espaços distintos do mesmo local respeitam o deslocamento interno; em locais distintos respeitam o deslocamento externo.

2. **Atomicidade Transacional (Zero Overbooking)**:
   - A checagem de saldo disponível na janela efetiva (incluindo buffers) e a gravação da reserva ocorrem estritamente na **mesma transação** no banco de dados.
   - No SQLite via `BEGIN IMMEDIATE`; no PostgreSQL via `SELECT ... FOR UPDATE` / `SERIALIZABLE` com retry para `40001`.

3. **Alocação de Espaços e Sublocação**:
   - Suporte a reserva de `equipment_model` (com alocação automática ou manual de unidades/patrimônios), expansão de kits e sinalização de déficit para **sublocação** junto a fornecedores cadastrados.

---

## 5. Orçamentos, Pedidos e Operação de Campo (Fase 4)

1. **Orçamentos e Pedidos**:
   - Fluxo de estados: `Rascunho -> Enviado -> Aprovado (gera reserva firme) -> Em Separação -> Em Evento -> Devolvido -> Concluído` (ou `Cancelado` / `Expirado`).
   - Precificação por diária com tabelas de desconto por quantidade de dias (ex.: fator progressivo), descontos/acréscimos controlados por permissão e valores sempre em centavos.
   - Termos comerciais, vínculo obrigatório com Cliente, Local (`venue` + `venue_space`) e Contato do evento.

2. **Separação, Check-out, Check-in e Manutenção**:
   - Geração de romaneio (lista de separação expandida com peso total e carga elétrica estimada).
   - Conferência por leitura de etiqueta/código de barras ou manual na saída e no retorno.
   - Registro imediato de avarias, faltas ou extravios no retorno, abrindo ordem de manutenção e bloqueando a unidade avariada da disponibilidade futura até conclusão do reparo.

---

## 6. Financeiro, Sublocação e Relatórios (Fase 4.5)

1. **Contas a Receber e Pagamentos**:
   - Parcelamento/condições de pagamento vinculadas ao pedido aprovado, baixas parciais/totais, registro de caução/sinal e inadimplência.
2. **Contas a Pagar e Sublocação**:
   - Registro de custos de sublocação vinculados a fornecedores (`is_supplier = true`) e despesas operacionais do evento.
3. **Auditoria e Indicadores**:
   - Trilha de auditoria imutável (`audit_log`) em todas as operações críticas, respeitando mascaramento LGPD de dados pessoais.
   - Relatórios de ocupação/ROI por modelo de equipamento, faturamento por período e histórico de manutenção.
