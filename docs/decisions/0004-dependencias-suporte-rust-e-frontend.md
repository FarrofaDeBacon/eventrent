# ADR-0004: Dependências Auxiliares do Ecossistema Rust e Frontend

## Contexto

A definição da stack principal lista os frameworks e bibliotecas de alto nível (SvelteKit 2, Svelte 5, Tailwind 4, bits-ui, Tauri 2, SQLx, Vitest, Playwright, ESLint, Prettier). Para viabilizar as regras inegociáveis de arquitetura (UUID v7, datas UTC ISO-8601, tratamento de erros tipado sem `unwrap()`, runtime assíncrono para o SQLx, testes de componentes no DOM e composição segura de classes Tailwind), algumas bibliotecas de base do ecossistema são tecnicamente indispensáveis.

## Decisão

Autorizar e documentar o uso das seguintes dependências de suporte estritamente acopladas aos itens da stack:

### Backend (Rust)

- `tokio`: Runtime assíncrono exigido pelo `sqlx` (`runtime-tokio`) e pelo `tauri`.
- `serde` e `serde_json`: Serialização de structs entre `crates/core` e a camada IPC do Tauri.
- `uuid` (com feature `v7`): Geração de identificadores UUID v7 ordenáveis exigidos pela Regra 3.
- `chrono`: Geração e formatação de timestamps UTC em ISO-8601 exigidos pela Regra 4.
- `thiserror`: Definição ergonômica de erros fortemente tipados em `crates/core` exigida pela Regra 9.
- `tracing` e `tracing-subscriber`: Diagnóstico estruturado de inicialização e migrações.
- `tauri-plugin-opener`: Plugin oficial do Tauri 2 para abertura segura de links/arquivos no sistema operacional.

### Frontend (TypeScript / Svelte)

- `@tauri-apps/api` e `@tauri-apps/cli`: SDK oficial e CLI do Tauri 2 para comunicação via `invoke` e build desktop.
- `clsx` e `tailwind-merge`: Composição de classes utilitárias sem conflito de especificidade (padrão oficial do `shadcn-svelte` / `bits-ui`).
- `jsdom` e `@testing-library/jest-dom`: Ambiente DOM simulado e asserções para execução do `@testing-library/svelte` no `vitest`.
- `@eslint/js`, `typescript-eslint`, `eslint-plugin-svelte`, `eslint-config-prettier`, `globals`: Plugins necessários para que o ESLint 9 processe arquivos TypeScript e Svelte 5.

## Consequências

- Positivas: Cumprimento integral das regras de arquitetura sem reinventar primitivas críticas (como gerador de UUID v7 ou parser ISO-8601).
- Negativas/Atenção: Qualquer nova dependência futura fora deste escopo continuará exigindo um novo ADR específico.
