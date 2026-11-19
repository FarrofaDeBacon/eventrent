# 05 - Roadmap de Evolução: EventRent

Este roadmap detalha as fases oficiais de evolução do sistema EventRent definidas pela Direção do Projeto.

---

### Fase 0 — Fundação Arquitetural (Concluída — `v0.1.0-fase0`)

- Scaffold do projeto (Tauri 2 + SvelteKit static + TypeScript strict + Tailwind 4 + Cargo workspace).
- Governança de documentação (`AGENTS.md`, `docs/`, ADRs 0001 a 0005).
- Design system base com tokens OKLCH, alternância de tema e injeção dinâmica de branding.
- Componentes base de UI (Button, Input, Card, Badge, Dialog, Toast, Estados).
- App Shell (sidebar recolhível, cabeçalho, paleta de comandos Ctrl+K, tela de Configurações > Aparência).
- Suporte a i18n (`pt-BR`).
- Logs em arquivo com rotação diária, instância única e diálogo nativo de erro no boot.
- Testes automatizados e CI verde em Ubuntu e Windows.

---

### Fase 1A — Fundações de Dados, Usuários, Permissões e Escolha de Modo (Escopo Atual)

- Refatoração arquitetural de repositórios por trait (`crates/core` sem `sqlx` + `crates/store-sqlite` + suíte genérica de testes de contrato).
- Escolha do modo de operação na primeira execução (`config.json` fora do banco): Modo Local ativo e Modo Nuvem sinalizado como "Em breve".
- Assistente de primeira execução no modo local com criação da conta do Proprietário e geração de Chave de Recuperação única.
- Controle de acesso baseado em papéis e permissões (`resource:action`) verificado no `core`, com perfis de sistema (`Proprietário`, `Comercial`, `Estoque`, `Técnico`, `Financeiro`).
- Senhas com Argon2id, bloqueio temporário progressivo após falhas de login, expiração de sessão por inatividade e trilha de auditoria (`audit_log`).
- Telas de Login, Recuperação de Senha, Gestão de Usuários e Consulta de Perfis.

---

### Fase 1B — Clientes e Marca Completa (Concluída — `v0.5.0-fase1b2`)

- Cadastro completo de Clientes (PJ e PF) com documentos (CPF e CNPJ numérico/alfanumérico conforme IN RFB nº 2.229/2024), endereços opcionais, contatos obrigatórios, busca indexada sem acentos e lista virtualizada.
- Validação estrita de branding (hexadecimal, dimensões, imagens), upload local de logotipo e geração automática de escala tonal OKLCH com verificação de contraste WCAG AA.

---

### Fase 1C — Fornecedores, Correções de Hardening e ADR-0012 (Escopo Atual)

- Normalização de telefones em E.164 (`+`), validação ASCII estrita de e-mails, geração de IDs pelo servidor e limites Unicode em nomes de usuários.
- Cadastro unificado de terceiros (`customer`) com papéis (`is_customer`, `is_supplier`), categorias de fornecedor (`sublocacao`, `manutencao`, `transporte`, `compras`, `servicos`, `outros`) e permissões granulares `supplier:read` / `supplier:manage`.
- Registro arquitetural do modelo de alocação, capacidade, locais/espaços e equipe no **ADR-0012**.

---

### Fase 1D — Locais de Evento com Espaços

- Cadastro de Locais (venues) e seus respectivos Espaços (pavilhões, salas, auditórios), com tempo de deslocamento interno configurável por local e campos técnicos opcionais (pé-direito, potência elétrica, rigging, acesso de carga, horários de montagem e restrições de ruído).

---

### Fase 2 — Catálogo e Estoque com Capacidade

- Modelos de equipamentos com atributos técnicos do audiovisual e conceito unificado de capacidade (ADR-0012).
- Modelo híbrido de controle de estoque: unidade com patrimônio (`serialized`, capacidade = 1) vs. item por quantidade (`quantity`, capacidade = saldo).
- Cadastro de unidades patrimoniais, números de série e composição de Kits.

---

### Fase 2.5 — Servidor Axum, PostgreSQL e Modo Nuvem

- Criação do `crates/store-postgres` implementando as traits de store do `crates/core` e validado pela mesma suíte de contrato.
- Servidor HTTP (`axum`) expondo os serviços do `crates/core`.
- Implementação HTTP em `src/lib/data` e ativação da opção "Em mais de um computador (Nuvem)" no assistente inicial.

---

### Fase 2.6 — Assistente de Migração Local para Nuvem

- Ferramenta guiada para exportar/sincronizar os dados de uma instalação SQLite local (preservando UUIDs v7, `tenant_id` e histórico de auditoria) para o servidor PostgreSQL em nuvem.

---

### Fase 3 — Motor de Disponibilidade, Reservas e Timeline

- Motor único de disponibilidade por intervalo de tempo e capacidade, aplicando os 3 níveis de intervalos (virada no mesmo espaço 30 min, deslocamento interno configurável por local 20 min, deslocamento externo 60 min) e sugestão de alternativas.
- Visualização de Timeline interativa e Gantt de locação por equipamento/espaço/categoria.

---

### Fase 4 — Orçamento e Contrato

- Criação e versionamento de propostas comerciais e orçamentos com hierarquia `Cliente → Evento → Ambientes → Alocações`.
- Geração de contratos em PDF a partir de templates customizáveis.

---

### Fase 4.5 — Equipe Técnica

- Cadastro de profissionais (fixo/freelancer) com capacidade padrão 1 ou liberação de simultaneidade N, funções, certificações com validade, rateio de custo configurável, níveis de risco por local/horário com justificativa e auditoria, e login opcional.

---

### Fase 5 — Conferência e Operação com QR Code

- Romaneio de saída, leitura/conferência rápida via QR Code / código de barras, registro de avarias e checklist de retorno.

---

### Fase 6 — Financeiro

- Contas a pagar e receber integradas às locações, controle de caução, fluxo de caixa e conciliação.

---

### Fase 7 — Fiscal por Terceiro

- Emissão de notas fiscais e documentos de remessa via integração com APIs fiscais terceirizadas.

---

### Fase 8 — Backup, Atualização e Licenciamento

- Rotinas de backup local/nuvem, atualização automática segura e controle de licenciamento.

---

### Fase 9 — Produção Online

- Operação SaaS multi-tenant completa em produção online com observabilidade, escalabilidade e provisionamento automatizado.

---

### Detalhamento Fases 2A a 4.5 (Alinhado a `docs/06-requisitos-de-produto.md`)

- **Fase 2A (Concluída — `v0.8.0-fase2a`)**: Catálogo de Modelos de Equipamentos, Marcas, Árvore de Categorias (até 4 níveis) com Herança de Campos Técnicos, Carga Sugerida AV idempotente e RBAC (`catalog:read` / `catalog:manage`).
- **Fase 2M (Concluída nesta entrega)**: Endurecimento do Catálogo (Tarefa 0 da 2A) e Mídia de Modelos de Equipamentos (ADR-0013, Migração `0009_media.sql`, `MediaStore` + `crates/media-fs` endereçado por SHA-256, rejeição de EXIF, galeria de fotos com redimensionamento WebP/JPEG no navegador e colagem `Ctrl+V`, documentos técnicos PDF abertos fora do app, miniatura na listagem virtualizada, importação em lote de fotos em `/catalog/import-photos` e coleta de lixo manual).
- **Fase 2B (Próxima)**: Estoque Físico (unidades serializadas com patrimônio/QR e saldos por quantidade, coluna de unidade de medida no modelo e trava de `control_mode` quando existirem unidades) e Depósitos/Localizações.
- **Fase 2C**: Kits e Composição de Equipamentos (kits expansíveis, acessórios vinculados e regras de capacidade de conjunto).
- **Fase 2.5**: Portabilidade Multi-Usuário e Nuvem (`crates/store-postgres`, Object Storage para `MediaStore` com varredura antivírus, sessões server-side, rate limiting anti-DoS de lockout).
- **Fase 3**: Motor de Disponibilidade e Reservas atômicas com Modelo de Três Tempos (ADR-0012), alocação de espaços e sinalização de sublocação.
- **Fase 4**: Orçamentos, Pedidos, Romaneio de Separação, Check-out/Check-in por código de barras e Manutenção.
- **Fase 4.5**: Financeiro (Contas a Receber/Pagar, Sublocação) e Relatórios Gerenciais de Ocupação/ROI.
