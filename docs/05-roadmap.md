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

### Fase 1B — Clientes e Marca Completa

- Cadastro completo de Clientes (PJ e PF) com documentos, endereços, contatos e histórico.
- Validação estrita de branding (hexadecimal, tamanhos, sanitização de SVG), upload local de logotipo e geração automática de escala tonal com verificação de contraste WCAG AA.

---

### Fase 2 — Catálogo de Equipamentos e Estoque

- Modelos de equipamentos com atributos técnicos do audiovisual.
- Modelo híbrido de controle de estoque (`serialized` vs. `quantity`).
- Cadastro de unidades patrimoniais e números de série.
- Composição de Kits que se desdobram em itens componentes.

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

- Algoritmo de cálculo de disponibilidade e gravação de reservas na mesma transação.
- Visualização de Timeline interativa e Gantt de locação por equipamento/categoria.
- Gestão de períodos: trânsito, montagem, evento, desmontagem e retorno.

---

### Fase 4 — Orçamento, Contrato e Emissão de Documentos

- Criação e versionamento de propostas comerciais e orçamentos.
- Geração de contratos em PDF a partir de templates customizáveis.

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
