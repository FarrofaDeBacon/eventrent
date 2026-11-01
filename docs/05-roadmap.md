# 05 - Roadmap de Evolução: EventRent

Este roadmap detalha as fases de evolução do sistema EventRent definidas pela Direção do Projeto.

---

### Fase 0 — Fundação Arquitetural (Escopo Atual)
- Scaffold do projeto (Tauri 2 + SvelteKit static + TypeScript strict + Tailwind 4 + Cargo workspace).
- Governança de documentação (`AGENTS.md`, `docs/`, ADRs iniciais).
- Design system base com tokens OKLCH, alternância de tema e injeção dinâmica de branding.
- Componentes base de UI (Button, Input, Card, Badge, Dialog, Toast, Estados).
- App Shell (sidebar recolhível, cabeçalho, paleta de comandos Ctrl+K, tela de Configurações > Aparência).
- Suporte a i18n (`pt-BR`).
- Infraestrutura de banco de dados SQLite via `sqlx` no `crates/core`, migrações automáticas, tabelas `tenant` e `tenant_branding`.
- Comunicação ponta a ponta (Tela -> `src/lib/data` -> `invoke` -> `crates/core` -> SQLite).
- Bateria de testes (Rust in-memory, Vitest mock data, Playwright smoke test) e CI no GitHub Actions.

---

### Fase 1 — Base e Controle de Acesso
- Gestão completa de Tenant.
- Usuários, autenticação local e perfis de permissão (RBAC).
- Cadastro de Clientes (PJ e PF) com histórico e contatos.
- Branding avançado com geração automática de escala tonal e verificação WCAG AA.

---

### Fase 2 — Catálogo de Equipamentos e Estoque
- Modelos de equipamentos com atributos técnicos específicos do audiovisual.
- Modelo híbrido de controle de estoque (`serialized` vs. `quantity`).
- Cadastro de unidades patrimoniais e números de série.
- Composição de Kits que se desdobram em itens componentes.
- Campos personalizados e anexos técnicos por modelo.

---

### Fase 3 — Motor de Disponibilidade, Reservas e Timeline
- Algoritmo de cálculo de disponibilidade transacional no backend Rust.
- Reservas e alocações atômicas (prevenção de conflito de datas e overbooking).
- Visualização de Timeline interativa e Gantt de locação por equipamento/categoria.
- Gestão de períodos: trânsito, montagem, show/evento, desmontagem e retorno.

---

### Fase 4 — Orçamento, Contrato e Emissão de Documentos
- Criação e versionamento de propostas comerciais e orçamentos.
- Tabela de preços dinâmicos, descontos e acréscimos.
- Geração de contratos em PDF a partir de templates editáveis.
- Assinatura digital e fluxo de aprovação do cliente.

---

### Fase 5 — Expedição, Conferência e Devolução (Logística)
- Romaneio de carga para saída do galpão.
- Leitura e conferência rápida com QR Code / código de barras.
- Registro de avarias, perdas e substituição emergencial de equipamentos.
- Checklist de inspeção no retorno do evento e baixa no estoque.

---

### Fase 6 — Financeiro e Cobrança
- Contas a pagar e contas a receber integradas às locações.
- Fluxo de caixa previsto x realizado.
- Controle de caução, faturamento parcelado e conciliação bancária (PIX, boleto, cartão).

---

### Fase 7 — Integração Fiscal
- Emissão de notas fiscais de serviço e remessa de locação por meio de integração com serviços terceirizados (APIs fiscais).

---

### Fase 8 — Confiabilidade, Atualização e Licenciamento
- Rotinas de backup automatizado (local e nuvem).
- Sistema de atualização automática segura (Tauri updater).
- Gestão de chaves de licença e planos do produto SaaS desktop.

---

### Fase 9 — Versão Online (SaaS Web / Nuvem)
- Implementação de transporte HTTP em `src/lib/data` para conexão com backend em nuvem sem modificar o frontend.
- Migração do `crates/core` para execução como serviço de API em servidor (PostgreSQL).
- Modo híbrido com sincronização offline para o aplicativo desktop de galpão.
