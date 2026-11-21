# ADR-0016: Contratos IPC entre Rust e TypeScript, Sessão Segura e Isolamento

## Status

Aceito (Fase 2S - Auditoria de Segurança e Confiabilidade)

## Contexto

O EventRent é uma aplicação desktop offline-first construída com Tauri v2, SvelteKit (Svelte 5 Runes) e SQLite via SQLx.
A comunicação entre a interface do usuário (TypeScript no Webview) e as regras de negócio / banco de dados (Rust no processo Core) ocorre primariamente via IPC assíncrono do Tauri (`invoke` de comandos e envio de frames binários de mídia).

Durante a auditoria de segurança e revisão da Fase 2M, foram identificadas fragilidades nos contratos entre o frontend e o backend:

1. **Desalinhamento silencioso de DTOs e assinaturas:** Métodos no TypeScript podiam descartar dados retornados pelo Rust (por exemplo, `recoverPasswordWithKey` tipado como `Promise<void>` descartando o `new_recovery_key` retornado pelo Rust).
2. **Falta de tipagem compartilhada estrita:** Alterações nas structs de Rust nem sempre geravam falhas em tempo de compilação ou teste no TypeScript.
3. **Invalidação de sessão incompleta:** Sessões mantidas em memória no frontend ou instâncias clonadas não eram invalidadas quando uma senha era trocada, uma conta desativada ou uma chave de recuperação regenerada.
4. **Política de Segurança de Conteúdo (CSP):** CSP permissiva com scripts inline representava risco de injeção e execução não autorizada.

## Spike: Avaliação de Alternativas para Contratos e Tipos IPC

Avaliamos três abordagens para garantir consistência absoluta entre Rust e TypeScript:

### 1. `ts-rs`

- **Como funciona:** Derive macro `#[derive(TS)]` nas structs Rust que gera arquivos `.d.ts` ou `.ts` durante compilação ou execução de testes `cargo test`.
- **Vantagens:** Gera tipos TypeScript fiéis aos tipos Rust com `serde`; não interfere no runtime do Tauri; leve e decoupled.
- **Desvantagens:** Não gera assinaturas dos comandos Tauri invocáveis (somente DTOs de dados); requer adicionar anotações em dezenas de structs do `core` e acoplar o `core` com crates adicionais de build.

### 2. `tauri-specta`

- **Como funciona:** Gera tanto as definições de tipo TypeScript quanto as funções de chamada tipadas para `invoke` do Tauri 2 a partir do registro de handlers Rust.
- **Vantagens:** Tipagem ponta a ponta dos comandos e dos parâmetros de entrada/saída.
- **Desvantagens:** No ecossistema Tauri v2 (RC / estável recente), plugins e handlers binários personalizados (como o parser de frames binários `decode_photo_ipc_frame` e `decode_document_ipc_frame`) exigem adaptações complexas; acopla fortemente a camada de dados do frontend ao runtime Tauri, violando a arquitetura hexagonal e o padrão de repositórios que permite execução em ambiente Web de desenvolvimento com mocks puros (`mock-*.ts`).

### 3. Fixtures Canônicas de Contrato com Validação Bidirecional no Rust e Vitest (Abordagem Adotada)

- **Como funciona:**
  1. No Rust (`src-tauri/src/contracts.rs`), um gerador/validador automatizado itera sobre `COMMAND_REGISTRY` (garantido sincronizado com `generate_handler!`). Para cada comando, há uma fixture JSON versionada em `src/lib/data/fixtures/contracts/<comando>.json` contendo o nome, escopo (`is_public`), flag `has_return_value`, `sample_output` e `sample_errors`.
  2. Um teste unitário Rust (`test_all_commands_have_up_to_date_contract_fixtures`) falha no CI se houver qualquer comando sem fixture, com contrato divergente ou qualquer fixture órfã. A geração/atualização é habilitada explicitamente via flag `EVENTRENT_UPDATE_CONTRACTS=1`.
  3. No frontend (`src/lib/data/contracts.spec.ts`), uma suíte Vitest lê as fixtures e valida que nenhum comando com retorno está tipado como `Promise<void>`, que os DTOs do TypeScript (`types.ts`) contêm todas as chaves e tipos esperados pelo Rust, e valida os retornos críticos (`recover_password` e `regenerate_recovery_key` com `new_recovery_key`).
- **Vantagens:**
  - Zero dependências invasivas no core do Rust.
  - Independência total da arquitetura hexagonal: o frontend continua usando interfaces abstratas (`AuthRepository`, `CatalogRepository`, etc.) e implementações desacopladas (`TauriAuthRepository`, `MockAuthRepository`).
  - Impossibilidade de drift: se o backend alterar o retorno de um comando sem atualizar a fixture ou o frontend, o CI falha no Rust e no TypeScript.
  - Sem quebras com frames binários ou plugins de terceiros.

## Decisão

1. **Estratégia de Contratos IPC:**
   Adotar as **Fixtures Canônicas de Contrato** versionadas em JSON (`src/lib/data/fixtures/contracts/*.json`), validadas de forma bidirecional pelo `cargo test` no Rust e pelo `vitest` no TypeScript.
   - Roadmap futuro (Fase 3+): Avaliar adoção de `ts-rs` exclusivamente para gerar types de envelopes comuns, mantendo a validação por fixtures como guarda de regressão do runtime.

2. **Revalidação de Sessão por Época (`auth_epoch`):**
   - A tabela `app_user` recebe a coluna `auth_epoch INTEGER NOT NULL DEFAULT 0` via migração congelada `0010_auth_epoch.sql`.
   - Toda alteração de credencial (troca de senha, recuperação de senha, regeneração de chave, alteração de role, desativação) incrementa atomicamente o `auth_epoch` do usuário.
   - O `SessionInfo` armazena o `auth_epoch` gerado no login.
   - O Core, a cada comando protegido, recarrega o estado atual do usuário no banco SQLite, checando se a conta está ativa e se `session.auth_epoch == user.auth_epoch`. Qualquer divergência invalida a sessão imediatamente com erro `Unauthorized`, impedindo uso de sessões clonadas ou obsoletas.
   - As permissões do usuário são recarregadas dinamicamente do banco a cada comando, eliminando janelas de privilégio estático.

3. **Isolamento de Scripts e CSP Estrita:**
   - Remoção de qualquer script inline no HTML (`app.html`), transferindo a rotina anti-FOUC de tema para arquivo estático servido localmente (`static/anti-fouc.js`).
   - CSP estrita em `tauri.conf.json`:
     `default-src 'self'; script-src 'self'; img-src 'self' blob: data:; font-src 'self'; connect-src ipc: http://ipc.localhost; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'none'; style-src 'self' 'unsafe-inline'`
   - **Justificativa de `'unsafe-inline'` em `style-src`:** O motor de renderização do Svelte 5 e a biblioteca de utilitários Tailwind CSS (com tema dinâmico OKLCH) exigem injeção de propriedades customizadas reativas via atributos `style="..."` nos elementos HTML (por exemplo, `style="--primary: ..."` e variáveis dinâmicas de cores/dimensões). Diferente de scripts executáveis, estilos inline não permitem execução arbitrária de código, enquanto `script-src` permanece estritamente `'self'`, bloqueando totalmente scripts inline, avaliadores (`'unsafe-eval'`) e fontes externas não confiáveis.
   - Teste automatizado no Rust (`guards.rs`) para impedir regressões na CSP (garantindo que não seja nula nem vazia, proibindo `'unsafe-eval'` e curingas em `script-src`, e validando a presença de `style-src 'self' 'unsafe-inline'`).

4. **Isolamento de Mocks fora do Pacote de Produção:**
   - Todas as 7 implementações mockadas (`mock-*.ts`) contêm guards explícitos `if (!import.meta.env.DEV && import.meta.env.VITE_E2E !== "1") throw new Error(...)`, impedindo estritamente sua execução fora do ambiente de desenvolvimento ou testes.
   - Em `src/lib/data/index.ts`, a fábrica de repositórios valida o ambiente via `requireDesktopOrDev`. Caso executada em produção fora do runtime Tauri, ela lança erro imediato em vez de instanciar mocks silenciosamente.
   - Na camada de apresentação (`src/routes/+layout.svelte`), o acesso web de produção renderiza uma tela bloqueante com instrução explícita ("Execute o aplicativo desktop EventRent"). Em modo de desenvolvimento no navegador (mocks ativos), exibe uma tarja visual persistente e não-armazenada em storage ("MODO DE DEMONSTRAÇÃO: Não use dados reais") com semântica de alerta acessível.

5. **Estratégia de Testes Automatizados E2E e Avaliação de `tauri-driver`:**
   - **Spike sobre `tauri-driver` vs Playwright:**
     - O `tauri-driver` depende de drivers WebDriver nativos por plataforma (WebKitWebDriver no Linux via WebKit2GTK e msedgedriver no Windows). Em ambientes de integração contínua (CI headless), o suporte a WebKit2GTK em modo headless é propenso a instabilidades gráficas, dependências de display virtual (Xvfb) e incompatibilidades de versão do runtime nativo. Além disso, compilar o binário completo do Tauri em modo release antes de cada teste E2E adiciona mais de 10 a 15 minutos em cada execução do CI.
     - **Estratégia Adotada (Pirâmide de Testes Desacoplada):**
       1. **Interface e Fluxos de Usuário:** Cobertos por Playwright E2E executando Chromium headless contra a aplicação web servida com mocks em memória. Isso garante validação rápida (< 1 minuto), determinística e cross-platform em runners `ubuntu-latest` e `windows-latest`.
       2. **Comandos IPC, Segurança e Sessão:** Cobertos por suíte de testes de integração em Rust em `src-tauri/src/guards.rs` e `src-tauri/src/contracts.rs`, que validam autorização, expiração de inatividade, decodificação de frames binários, capabilities de menor privilégio e conformidade estrita de DTOs para todos os 62 comandos.
       3. **Domínio e Banco de Dados:** Cobertos por suíte completa de testes no SQLite (`store-sqlite`), transações atômicas com concorrência real, reversão e provas de idempotência.

## Consequências

- **Positivas:**
  - Garantia formal de sincronia entre os 62 comandos do Tauri e as camadas TypeScript.
  - Eliminação de falhas críticas de descarte de segredos (como chave de recuperação).
  - Sessões comprometidas ou desativadas são revogadas instantaneamente em todos os nós/janelas.
  - Mitigação de ataques XSS através de CSP sem scripts inline ou eval.
- **Negativas / Cuidados:**
  - Adicionar ou alterar um comando agora exige rodar `EVENTRENT_UPDATE_CONTRACTS=1 cargo test -p eventrent-app` e atualizar as interfaces TypeScript correspondentes.
