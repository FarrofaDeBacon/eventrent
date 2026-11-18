# ADR-0012: Modelo de Alocação, Capacidade, Locais/Espaços e Equipe

- **Status**: Aceito (Documento de Arquitetura — implementação distribuída nas Fases 1D a 4.5)
- **Data**: 2026-11-17
- **Fase**: 1C

## Contexto

Com a conclusão do cadastro unificado de clientes e fornecedores (Fases 1B.2 e 1C), as próximas etapas do EventRent envolvem locais de evento, catálogo e estoque, motor de disponibilidade, orçamentos/contratos e gestão de equipe técnica. Para evitar retrabalho de modelagem entre módulos, a Direção do Projeto consolidou as decisões estruturais de alocação de recursos (equipamentos e pessoas), locais e espaços (como pavilhões de centros de convenções, salas e auditórios) e regras de conflito e deslocamento.

## Decisão

### 1. Hierarquia de Cadastro e Operação

- **Hierarquia de Cadastro (Locais e Espaços):**
  - `Local (venue)` contém `Espaços` (pavilhões, salas, auditórios, palcos fixos).
  - Exemplo prático: um centro de convenções (ex.: Riocentro) é um `Local`, e seus pavilhões/auditórios são `Espaços` dentro dele.
  - O cadastro de `Espaço` possui campos técnicos opcionais em texto livre: pé-direito, potência elétrica, rigging, acesso e dimensões de carga, horários de montagem e restrições de ruído — com suporte previsto a anexos (plantas/fichas técnicas) futuramente.
- **Hierarquia de Operação:**
  - `Cliente → Evento → Ambientes → Alocações de recursos`.
  - Cada `Ambiente` aponta para um `Local` e um `Espaço`.
  - Todo evento nasce com um **ambiente padrão**, que permanece oculto na interface quando o evento possui apenas um ambiente (mantendo a UX simples para eventos menores, sem perder o modelo unificado para eventos multi-sala).

### 2. Recurso com Capacidade e Motor Único de Disponibilidade

- Todo recurso alocável opera sob o conceito unificado de **capacidade**:
  - **Unidade com patrimônio (`serialized`):** capacidade = `1`.
  - **Item por quantidade (`quantity`):** capacidade = saldo disponível em estoque.
  - **Pessoa (equipe):** capacidade = `1` por padrão, podendo ser `N` quando liberado explicitamente no cadastro do profissional.
- **Motor Único:** Um único motor de disponibilidade calcula conflitos e saldos tanto para equipamentos quanto para pessoas.

### 3. Alocação por Intervalo de Tempo e Três Níveis de Tempos

- As alocações são registradas por **intervalo de tempo** (cobrindo turnos de montagem, operação e desmontagem), e não por diária inteira.
- Os tempos de intervalo exigidos entre alocações funcionam em **três níveis**:
  1. **Virada no mesmo espaço:** padrão de `30 min` (configurável).
  2. **Deslocamento interno entre espaços do mesmo local:** padrão de `20 min`, configurável por local.
  3. **Deslocamento externo entre locais diferentes:** padrão de `60 min` (configurável).
- **Conflito de Equipamento:** A mesma unidade patrimonial alocada no mesmo horário continua sendo bloqueada como conflito, oferecendo sugestão de alternativas: outra unidade disponível do mesmo modelo, sublocação via fornecedor (categoria `sublocacao` da Fase 1C) ou reorganização de horário.

### 4. Níveis de Risco para Alocação de Pessoas

- **Mesmo espaço ou espaços do mesmo local com horários diferentes:** **Livre** (respeitando os tempos de virada/deslocamento interno).
- **Mesmo local com horário sobreposto:** **Aviso** (requer liberação de simultaneidade no cadastro da pessoa).
- **Locais diferentes sem tempo de deslocamento externo suficiente:** **Aviso de viabilidade**.
- **Locais diferentes com horário sobreposto:** **Exceção crítica** — exige liberação no cadastro da pessoa, **permissão específica** do operador, **justificativa** obrigatória e campo **"como será atendido"**, recebendo destaque visual no quadro operacional e registro na trilha de auditoria (`audit_log`).

### 5. Vínculo da Equipe, Custos e Disponibilidade

- **Vínculo:** Fixo (custo médio por dia) ou freelancer (diária por função).
- **Rateio:** Rateio de custo entre eventos simultâneos configurável.
- **Certificações:** Certificações (ex.: NR-10, NR-35) com data de validade bloqueiam atribuição a funções que as exijam quando vencidas.
- **Indisponibilidade:** Bloqueio de agenda/indisponibilidade sem registro de motivo detalhado (privacidade do colaborador).
- **Acesso ao sistema:** Login do técnico no sistema é opcional.

### 6. Recursos Futuros Previstos no Modelo

- Indicação **"já está no local"** ao montar evento subsequente no mesmo local/espaço.
- Geração de **tarefa de movimentação** para equipamento compartilhado entre espaços do mesmo local.
- **Linha do tempo do equipamento** (histórico contínuo de alocações, movimentações e manutenções por unidade).

### 7. Sequenciamento Oficial de Fases

- **Fase 1D:** Locais de evento com Espaços.
- **Fase 2:** Catálogo e estoque com capacidade.
- **Fase 3:** Motor de disponibilidade.
- **Fase 4:** Orçamento e contrato.
- **Fase 4.5:** Equipe.

## Consequências

- A Fase 1D prepara os cadastros de Locais e Espaços (incluindo tempo de deslocamento interno configurável por local e campos técnicos opcionais do espaço) antes da criação do catálogo/motor de disponibilidade.
- O conceito de capacidade e intervalo de tempo evita duplicar regras entre equipamentos e equipe técnica nas Fases 2, 3 e 4.5.
- Nenhuma tabela ou código deste ADR é implementado na Fase 1C; este documento serve de especificação formal para as fases seguintes.

## Adendo (Fase 1D — 2026-11-17)

- Implementado na Fase 1D (`0007_venues_and_settings.sql`):
  - `venue.internal_transfer_minutes` (`INTEGER NULL`, `0..=240` minutos; `NULL` = usar o padrão da empresa `allocation.default_internal_transfer_minutes`, padrão `20`) para modelar o tempo de deslocamento interno do local.
  - Chaves em `tenant_setting` para defaults globais do tenant:
    - `allocation.default_setup_minutes` (padrão `30`)
    - `allocation.default_teardown_minutes` (padrão `20`)
    - `allocation.default_turnover_buffer_minutes` (padrão `60`)
    - `allocation.default_internal_transfer_minutes` (padrão `20`)
  - Todos validados no intervalo `0..=240` minutos e editáveis em **Configurações → Operação** (`/settings/operation`).
  - Na Fase 4, espaços referenciados em eventos terão sua remoção bloqueada pelo domínio, orientando a desativação (`active = false`).
