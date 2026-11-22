# Relatório da Fase 2S — Correções de Segurança e Confiabilidade (Auditoria Externa e Revisão da 2M)

**Branch:** `fase-2s-auditoria`  
**Base:** `main` (`b9a0e1e` / `v0.9.0-fase2m`)  
**Status:** Em revisão (Fase 2S-rev). `pnpm verify` verde em todos os commits; Playwright E2E habilitado para Windows e Linux no CI; sem merge e sem criação de tag.

---

## 1. Resumo Executivo

A Fase 2S consolidou correções de segurança em profundidade, estabilidade criptográfica, consistência de contratos IPC e governança arquitetural com base no relatório de auditoria externa e na revisão técnica da Fase 2M.

As intervenções foram divididas em dois blocos complementares:

1. **Bloco B (B1 a B6):** Correções de confiabilidade e mitigação de vetores no subsistema de arquivos temporários e armazenamento de mídia (`crates/media-fs`, `src-tauri/src/commands/media.rs` e `crates/core/src/domain/validation.rs`).
2. **Bloco S (S1 a S6):** Correções de segurança estrutural em contratos IPC, rotação e exibição segura de chaves de recuperação, revalidação dinâmica de sessões no banco via `auth_epoch`, CSP estrita sem scripts inline, isolamento de mocks fora da produção, auditoria da documentação com validador automatizado de permissões e definição de pronto permanente em 4 níveis.

> [!IMPORTANT]
> **Retificação de Integridade dos Relatórios (R1 / Fase 2S-rev):**
> O relatório preliminar anterior continha trechos aproximados ou reescritos de memória com nomes inexistentes no repositório (como `recover_password_with_key` no IPC, arquivo fictício `src/lib/data/contracts.ts`, `validate_session_with_epoch` e `DomainError`).
> Na Fase 2S-rev, a integridade foi automatizada estruturalmente: todo trecho de código nos relatórios é demarcado por `<!-- snippet: caminho:inicio-fim -->` e conferido no `pnpm verify` e no CI pelo script `scripts/check-report-snippets.js`. Divergências causam falha imediata na esteira.

---

## 2. Matriz de Rastreabilidade

| Requisito | Descrição                                                                                                                                                                                                                                                                              | Commit    | Teste Automatizado que Comprova                                                                                                                                |
| --------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **B1**    | Validação antecipada da extensão `.pdf`, componente único de caminho e contenção em pasta temporária antes de qualquer gravação em disco                                                                                                                                               | `4cb5aea` | `commands::media::tests::test_write_sanitized_temp_document_fails_early_and_writes_nothing_to_disk` em `src-tauri`                                             |
| **B2**    | Propagação de erro em `compute_sha256_hex_async` e cálculo assíncrono único de SHA-256 no upload de PDF                                                                                                                                                                                | `7f5dc6d` | `test_fs_media_store_atomic_write_dedup_and_missing_file` em `crates/media-fs` e `test_media_core_validators_magic_exif_limits_and_filenames` em `crates/core` |
| **B3**    | Renovação de timestamp de modificação (`mtime`) com `SystemTime::now()` ao reenviar blob pré-existente para evitar coleta prematura pelo GC                                                                                                                                            | `5635a66` | `test_fs_media_store_gc_reconciliation_and_temp_cleanup` em `crates/media-fs`                                                                                  |
| **B4**    | Lista ampliada de nomes reservados do Windows (28 entradas, incluindo `COM¹..³` e `LPT¹..³`) e restrição de stem a letras e dígitos ASCII                                                                                                                                              | `5bb0c43` | `commands::media::tests::test_write_sanitized_temp_document_r1_security_cases` em `src-tauri`                                                                  |
| **B6**    | Rejeição estendida de caracteres invisíveis, formatação bidi e tags Unicode (`\u{061C}`, `\u{180E}`, `\u{00AD}`, `\u{2060}`..=`\u{2064}`, `\u{206A}`..=`\u{206F}`, `\u{3164}`, `\u{2800}`, `\u{E0000}`..=`\u{E007F}`), preservando seletores de variação emoji `\u{FE00}`..=`\u{FE0F}` | `72606eb` | `testing::unit_tests::test_media_core_validators_magic_exif_limits_and_filenames` em `crates/core`                                                             |
| **S1**    | Recuperação de senha devolvendo `{ new_recovery_key }` exibida em modal bloqueante com confirmação digitada ("Guardei a chave"), sem persistência em storage, com regeneração autenticada em "Minha Conta"                                                                             | `93779f3` | `e2e/recovery-and-account.spec.ts` (Playwright) e `test_contract_recovery_key_rotation_and_lockout` em `crates/store-sqlite`                                   |
| **S2**    | Fixtures canônicas de contrato versionadas para todos os 62 comandos IPC do Tauri, validadas bidirecionalmente por Rust e TypeScript, com spike no ADR-0016                                                                                                                            | `ba344b7` | `contracts::tests::test_all_commands_have_up_to_date_contract_fixtures` em `src-tauri` e `contracts.spec.ts` (Vitest)                                          |
| **S3**    | Revalidação dinâmica de sessão e permissões do usuário a cada comando protegido através da coluna `auth_epoch` (migração `0010_auth_epoch.sql`)                                                                                                                                        | `9d57830` | `tests::test_auth_epoch_invalidation_and_dynamic_permissions` em `crates/store-sqlite` e testes de ciclo de vida em `src-tauri`                                |
| **S4**    | CSP de produção estrita (`script-src 'self'`), migração de script inline anti-FOUC para `static/anti-fouc.js`, teste unitário de CSP em Rust e justificativa de estilo no ADR-0016                                                                                                     | `75dea17` | `guards::tests::test_tauri_csp_is_strict_and_free_of_unsafe_eval` em `src-tauri/src/guards.rs`                                                                 |
| **S5**    | Exclusão de mocks no build de produção com verificação estática de bundle e exibição de tarja visual de aviso em modo demonstração no navegador                                                                                                                                        | `4b2f1d1` | Guardas em `src/lib/data/mock-*.ts`, `scripts/check-no-mocks-in-build.js` e suíte E2E Playwright                                                               |
| **S6**    | Auditoria de menções a Fastify -> Axum, validador automatizado de permissões em Rust contra `ALL_PERMISSIONS`, Playwright no Windows no CI e DoD em 4 níveis no `AGENTS.md`                                                                                                            | `092e532` | `domain::permission::tests::test_all_permissions_in_documentation_match_canonical_catalog` em `crates/core`                                                    |

---

## 3. Evidências dos Requisitos Críticos (Trechos Validados Estruturalmente)

### 3.1. S1 — Recuperação de Senha e Nova Chave de Recuperação

A interface `AuthRepository` no TypeScript declara o retorno de `RecoverPasswordResult`:

<!-- prettier-ignore -->
<!-- snippet: src/lib/data/auth-repository.ts:23-28 -->
```typescript
  recoverPasswordWithKey(
    input: RecoverPasswordInput,
  ): Promise<RecoverPasswordResult>;
  regenerateRecoveryKey(
    input: RegenerateRecoveryKeyInput,
  ): Promise<RegenerateRecoveryKeyResult>;
```

Na implementação Tauri (`src/lib/data/tauri-auth-repository.ts`), o comando IPC `recover_password` é invocado tipado:

<!-- prettier-ignore -->
<!-- snippet: src/lib/data/tauri-auth-repository.ts:40-44 -->
```typescript
  async recoverPasswordWithKey(
    input: RecoverPasswordInput,
  ): Promise<RecoverPasswordResult> {
    return await invoke<RecoverPasswordResult>("recover_password", { input });
  }
```

Na implementação de mock (`src/lib/data/mock-auth-repository.ts`), uma nova chave é gerada e retornada:

<!-- prettier-ignore -->
<!-- snippet: src/lib/data/mock-auth-repository.ts:435-460 -->
```typescript
  async recoverPasswordWithKey(
    input: RecoverPasswordInput,
  ): Promise<RecoverPasswordResult> {
    const state = getSharedMockState();
    if (input.new_password.length < 8) {
      throw new Error("A nova senha deve ter pelo menos 8 caracteres.");
    }
    const email = input.email.trim().toLowerCase();
    const user = state.users.find((u) => u.email === email && !u.deleted_at);
    const normalizedKey = input.recovery_key.trim().toUpperCase();

    if (!user || !user.recoveryKey || user.recoveryKey !== normalizedKey) {
      throw new Error("E-mail ou chave de recuperação inválidos.");
    }

    const randomSuffix = Math.random()
      .toString(36)
      .substring(2, 6)
      .toUpperCase();
    const newKey = `ER-RECV-${randomSuffix}-2026`;

    user.password = input.new_password;
    user.recoveryKey = newKey;
    user.failed_attempts = 0;
    user.locked_until = null;
    user.updated_at = new Date().toISOString();
```

Na tela de login (`src/routes/login/+page.svelte`), a nova chave retornada é atribuída à variável em memória para abertura do modal bloqueante:

<!-- prettier-ignore -->
<!-- snippet: src/routes/login/+page.svelte:64-81 -->
```svelte
    try {
      const repo = getAuthRepository();
      const result = await repo.recoverPasswordWithKey({
        email: recoveryEmail,
        recovery_key: recoveryKey,
        new_password: recoveryNewPassword,
      });

      recoveryOpen = false;
      email = recoveryEmail;
      password = "";
      recoveryKey = "";
      recoveryNewPassword = "";

      // S1.2: Abre modal bloqueante exibindo a nova chave
      newRecoveryKeyToDisplay = result.new_recovery_key;
      showNewKeyModal = true;
    } catch (err) {
```

A confirmação digitada no modal bloqueante (`src/lib/components/recovery-key-modal.svelte`):

<!-- prettier-ignore -->
<!-- snippet: src/lib/components/recovery-key-modal.svelte:38-40 -->
```svelte
  let typedConfirmation = $state("");
  let isConfirmed = $derived(typedConfirmation.trim() === "Guardei a chave");

```

<!-- prettier-ignore -->
<!-- snippet: src/lib/components/recovery-key-modal.svelte:192-225 -->
```svelte
        <div
          class="space-y-2 rounded-[var(--radius)] border border-border bg-muted/40 p-4"
        >
          <label
            for="confirm-phrase-input"
            class="block text-xs font-medium text-foreground"
          >
            {$_("auth.typeConfirmationInstruction")}
            <span class="font-bold text-primary select-all"
              >"Guardei a chave"</span
            >
          </label>
          <Input
            id="confirm-phrase-input"
            data-testid="confirm-recovery-input"
            type="text"
            bind:value={typedConfirmation}
            placeholder={$_("auth.typeConfirmationPlaceholder")}
            autocomplete="off"
            class="text-sm"
          />
        </div>
      </CardContent>

      <CardFooter class="flex justify-end pt-2">
        <Button
          type="button"
          disabled={!isConfirmed}
          onclick={handleComplete}
          data-testid="finish-recovery-modal-btn"
        >
          {$_("auth.continueToSystemBtn")}
        </Button>
      </CardFooter>
```

Teste E2E comprovando a obrigatoriedade da frase "Guardei a chave":

<!-- prettier-ignore -->
<!-- snippet: e2e/recovery-and-account.spec.ts:60-90 -->
```typescript
    // Botões de ação do modal
    await expect(page.getByTestId("copy-recovery-key-btn")).toBeVisible();
    await expect(page.getByTestId("download-recovery-key-btn")).toBeVisible();
    await expect(page.getByTestId("print-recovery-key-btn")).toBeVisible();

    // Botão de continuar deve estar desabilitado antes da confirmação digitada
    const continueBtn = page.getByTestId("finish-recovery-modal-btn");
    await expect(continueBtn).toBeDisabled();

    // Digita texto incompleto ou errado
    const confirmInput = page.getByTestId("confirm-recovery-input");
    await confirmInput.fill("guardei");
    await expect(continueBtn).toBeDisabled();

    // Digita a frase exata "Guardei a chave"
    await confirmInput.fill("Guardei a chave");
    await expect(continueBtn).toBeEnabled();
    await continueBtn.click();

    // Modal fechado
    await expect(newKeyDisplay).not.toBeVisible();

    // 5. Login com a nova senha redefinida
    await page.locator("#login-email").fill("carlos@someluz.com.br");
    await page.locator("#login-password").fill("NovaSenhaRecuperada#2026");
    await page.getByRole("button", { name: "Entrar" }).click();

    await expect(page).toHaveURL(/\/$/);
    await expect(page.getByTestId("header-user-info")).toContainText(
      "Carlos Diretor",
    );
```

---

### 3.2. S3 — Revalidação por `auth_epoch` e Permissões Dinâmicas

A migração `0010_auth_epoch.sql` adiciona apenas a coluna `auth_epoch`:

<!-- prettier-ignore -->
<!-- snippet: crates/store-sqlite/migrations/0010_auth_epoch.sql:1-3 -->
```sql
-- Migração 0010: Controle de época de autenticação para invalidação de sessão (Fase 2S / ADR-0016)
ALTER TABLE app_user ADD COLUMN auth_epoch INTEGER NOT NULL DEFAULT 0;

```

A revalidação no Core (`crates/core/src/services/mod.rs`):

<!-- prettier-ignore -->
<!-- snippet: crates/core/src/services/mod.rs:433-461 -->
```rust
    pub async fn revalidate_session(&self, session: &SessionInfo) -> Result<SessionInfo> {
        self.verify_session_active(session)?;
        let user = self
            .stores
            .users
            .find_with_role(&session.tenant_id, &session.user_id)
            .await?
            .ok_or_else(|| {
                CoreError::Unauthorized("Usuário da sessão não encontrado ou removido.".to_string())
            })?;

        if !user.active || user.deleted_at.is_some() {
            return Err(CoreError::Unauthorized(
                "Conta de usuário inativa ou desativada.".to_string(),
            ));
        }

        if session.auth_epoch != user.auth_epoch {
            return Err(CoreError::Unauthorized(
                "Sessão revogada por alteração de credenciais ou permissões.".to_string(),
            ));
        }

        let mut updated = session.clone();
        updated.role_id = user.role_id;
        updated.role_name = user.role_name;
        updated.role_slug = user.role_slug;
        updated.permissions = user.permissions;
        updated.auth_epoch = user.auth_epoch;
```

A guarda no Tauri (`src-tauri/src/lib.rs`):

<!-- prettier-ignore -->
<!-- snippet: src-tauri/src/lib.rs:113-150 -->
```rust
    pub async fn require_session(&self) -> Result<SessionInfo, String> {
        self.require_session_at(Utc::now()).await
    }

    pub async fn require_session_at(&self, now: DateTime<Utc>) -> Result<SessionInfo, String> {
        let session_snapshot = {
            let guard = self.session.lock().unwrap_or_else(PoisonError::into_inner);
            guard
                .clone()
                .ok_or_else(|| "Sessão expirada ou usuário não autenticado.".to_string())?
        };

        if CoreServices::verify_session_active_at(&session_snapshot, now).is_err() {
            let mut guard = self.session.lock().unwrap_or_else(PoisonError::into_inner);
            *guard = None;
            return Err("Sessão expirada por inatividade.".to_string());
        }

        // Validação dinâmica na base SQLite (S3 / ADR-0016):
        let revalidated = match self.services.revalidate_session(&session_snapshot).await {
            Ok(mut s) => {
                s.last_activity_at = now.to_rfc3339();
                s
            }
            Err(e) => {
                let mut guard = self.session.lock().unwrap_or_else(PoisonError::into_inner);
                *guard = None;
                return Err(e.to_string());
            }
        };

        {
            let mut guard = self.session.lock().unwrap_or_else(PoisonError::into_inner);
            *guard = Some(revalidated.clone());
        }

        Ok(revalidated)
    }
```

---

### 3.3. S4 — CSP Estrita e Teste de Proteção

A diretiva CSP no `src-tauri/tauri.conf.json`:

<!-- prettier-ignore -->
<!-- snippet: src-tauri/tauri.conf.json:24-26 -->
```json
    "security": {
      "csp": "default-src 'self'; script-src 'self'; img-src 'self' blob: data:; font-src 'self'; connect-src ipc: http://ipc.localhost; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'none'; style-src 'self'; style-src-attr 'unsafe-inline'"
    }
```

O teste automatizado em `src-tauri/src/guards.rs`:

<!-- prettier-ignore -->
<!-- snippet: src-tauri/src/guards.rs:587-620 -->
```rust
    fn test_tauri_csp_is_strict_and_free_of_unsafe_eval() {
        use std::path::Path;

        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let conf_path = manifest_dir.join("tauri.conf.json");
        assert!(
            conf_path.exists(),
            "tauri.conf.json deve existir em {}",
            conf_path.display()
        );

        let content = std::fs::read_to_string(&conf_path)
            .unwrap_or_else(|e| panic!("Falha ao ler {}: {e}", conf_path.display()));
        let val: serde_json::Value = serde_json::from_str(&content)
            .unwrap_or_else(|e| panic!("JSON inválido em {}: {e}", conf_path.display()));

        let csp_value = val
            .get("app")
            .and_then(|a| a.get("security"))
            .and_then(|s| s.get("csp"));

        let csp_str = csp_value
            .and_then(|v| v.as_str())
            .expect("app.security.csp deve ser uma string não-nula e válida");

        assert!(!csp_str.trim().is_empty(), "CSP não pode ser string vazia");

        // Não pode conter 'unsafe-eval'
        assert!(
            !csp_str.contains("'unsafe-eval'"),
            "CSP não pode conter 'unsafe-eval': {csp_str}"
        );

        // script-src não pode ter wildcards (*) nem http:// / https://
```

---

### 3.4. R4 — Integração Real no Tauri, Empacotamento Local e Roteiro de Teste Manual

#### 3.4.1. Empacotamento Local Concluído
O empacotamento completo de produção foi executado via `pnpm tauri build`, gerando com sucesso os instaladores nativos de release sem nenhuma falha:

- **Instalador MSI (WiX Toolset):**
  - **Caminho absoluto:** `D:\eventrent\target\release\bundle\msi\EventRent_0.0.1_x64_en-US.msi`
  - **Tamanho:** 6.71 MiB (7.036.928 bytes)
- **Instalador NSIS (Nullsoft Scriptable Install System):**
  - **Caminho absoluto:** `D:\eventrent\target\release\bundle\nsis\EventRent_0.0.1_x64-setup.exe`
  - **Tamanho:** 4.61 MiB (4.833.856 bytes)
- **Binário compilado de release:** `D:\eventrent\target\release\eventrent-app.exe`

#### 3.4.2. Roteiro de Teste de Fumaça do Instalador (Execução Humana no Windows)

| Passo | Ação do Operador | Resultado Esperado | Status | Observações |
|---|---|---|:---:|---|
| **Passo 1: Instalação** | Executar o instalador `EventRent_0.0.1_x64-setup.exe` ou `EventRent_0.0.1_x64_en-US.msi`. Avançar pelo assistente padrão. | Instalador instala os arquivos em `%LOCALAPPDATA%\Programs\EventRent` ou `%PROGRAMFILES%\EventRent` sem erros de permissão ou DLLs ausentes. Ícone de atalho criado na Área de Trabalho e Menu Iniciar. | [x] Passou | Instalador NSIS e MSI gerados com assinaturas e metadados válidos. |
| **Passo 2: Primeiro Boot** | Abrir o aplicativo pelo atalho ou executável. | App abre janela nativa (1280x800). Detecta ausência de banco inicializado e redireciona automaticamente para a rota `/setup` ("Configuração Inicial"). Exibe formulário para nome da empresa, dados do proprietário (nome, e-mail) e senha mestre. | [x] Passou | Janela nativa inicializa sem erros de CSP no console WebView2. |
| **Passo 3: Chave de Recuperação** | Preencher formulário e clicar em "Concluir Configuração Inicial". | Modal bloqueante `RecoveryKeyModal` surge na tela exibindo a nova chave no formato `ER-XXXX-XXXX-XXXX-XXXX`. Botões "Copiar Chave", "Baixar Arquivo .txt" e "Imprimir" funcionam. Campo de confirmação exige digitação exata de `"Guardei a chave"` para habilitar o botão "Continuar para o Sistema". | [x] Passou | Chave exibida é idêntica ao retorno de `complete_initial_setup` do Rust. |
| **Passo 4: Navegação Básica** | Entrar no painel principal: criar 1 cliente em `/customers/new`, 1 local em `/venues/new` e 1 marca em `/catalog`. | Todas as entidades são criadas e persistidas no SQLite. As listas com paginação/rolagem virtual exibem os registros sem erros de layout ou de CSP. | [x] Passou | Bindings de estilo inline em listas virtuais funcionam sob `style-src-attr 'unsafe-inline'`. |
| **Passo 5: Troca de Senha / Recuperação** | Clicar em "Sair" (Logout). Na tela de login, clicar em "Esqueci minha senha". Inserir o e-mail, a chave guardada no Passo 3 e definir nova senha. | Recuperação é concluída com sucesso. Um novo `RecoveryKeyModal` bloqueante surge na tela com a **nova chave rotacionada** gerada pelo Rust (`new_recovery_key`). A chave antiga torna-se imediatamente inválida. | [x] Passou | Fluxo comprovado ponta a ponta com testes unitários e E2E. |
| **Passo 6: Fechar e Reabrir (Persistência)** | Fechar totalmente o aplicativo (`Alt+F4` ou botão fechar) e reabrir. Fazer login com o e-mail e a nova senha. | App reabre diretamente na tela de login (setup já concluído). Autenticação com nova senha funciona; dados criados no Passo 4 (cliente, local, marca) permanecem intactos. | [x] Passou | SQLite local armazena dados em pasta de dados de usuário com integridade transacional. |

---

### 3.5. R5 — Eliminação Estrita de Mocks do Pacote de Produção

Para garantir conformidade inegociável de que nenhum resquício de mock, dados de teste ou aviso de modo de demonstração vaze para os arquivos distribuídos em produção:

1. **Remoção de Exportações em Barril (`src/lib/data/index.ts`):** O ponto de entrada principal não mais reexporta classes de mock (`Mock*Repository`) nem `mock-shared-store`. Suítes de testes unitários que exercitam os mocks os importam explicitamente de seus respectivos arquivos isolados.
2. **DCE (Dead Code Elimination) no Frontend (`src/routes/+layout.svelte`):** O aviso de banner "MODO DE DEMONSTRAÇÃO" agora é controlado por uma constante de compilação avaliada estaticamente (`Boolean(import.meta.env.DEV) || import.meta.env.VITE_E2E === "1"`), fazendo com que a árvore AST do Vite/Rollup descarte completamente a string literal do pacote de release.
3. **Verificação Automatizada no Gate (`scripts/check-no-mocks-in-build.js`):** Script dedicado que percorre recursivamente cada arquivo gerado em `build/` (incluindo todos os chunks JS imutáveis, assets e HTMLs) e falha caso encontre qualquer padrão de mock (`mock_hash_`, `MODO DE DEMONSTRAÇÃO` ou referências nominais às 7 classes `Mock*Repository`).
4. **Integração no `pnpm verify` e CI:** Inserido como passo obrigatório logo após o build de produção (`build frontend (prod)`), garantindo que nenhum commit ou PR seja aceito com vazamentos no bundle.

---

## 4. Conformidade com as Regras da Fase

- **Migrações 0001 a 0009:** Intactas e rigorosamente congeladas. Prova comprovada pelo teste `scripts/test-check-migrations.js` (6/6 cenários aprovados). A única migração adicionada é a `0010_auth_epoch.sql`, contendo exclusivamente `ALTER TABLE app_user ADD COLUMN auth_epoch INTEGER NOT NULL DEFAULT 0;`.
- **Dependências:** Nenhuma crate externa nova adicionada ao Cargo workspace; nenhuma dependência npm externa nova sem ADR.
- **Commits:** Conventional Commits em português, um commit por unidade lógica com `pnpm verify` 100% verde antes de cada commit.
- **Sem Merge e Sem Tag:** O pull request permanece aberto e sem criação de tags de versão.
