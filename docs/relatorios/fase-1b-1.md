# Relatório de Conclusão: Fase 1B.1 — Correções Herdadas e Marca Completa (White Label)

**Data de Conclusão:** 09 de Novembro de 2026  
**Branch de Trabalho:** `fase-1b1-marca`  
**Pull Request Ativo:** [PR #5](https://github.com/FarrofaDeBacon/eventrent/pull/5)  
**Status do Pipeline:** Testes locais e CI verdes no Ubuntu e Windows

---

## 1. Sumário Executivo

A Fase 1B.1 implementou com sucesso o conjunto completo de correções herdadas da Fase 1B.0 (Tarefa 0: itens 0.1 a 0.9) e o sistema de Identidade Visual e White Label em sua totalidade (Tarefa 1: itens 1.1 a 1.9).

A arquitetura seguiu estritamente as diretrizes de portabilidade, separação de responsabilidades e segurança:

- **Resiliência Transacional**: O mecanismo genérico `run_in_tx` agora protege todo o ciclo de vida da transação no laço de retry, repetindo falhas transitórias em `begin_transaction` (como `SQLITE_BUSY` no `BEGIN IMMEDIATE`) e no `tx.commit()`.
- **Validação e Magic Bytes de Imagens**: Implementado o `BrandingValidator` no Core sem dependências externas, validando cores hexadecimais `#RRGGBB`, nomes de empresas e assinaturas mágicas para PNG (`89 50 4E 47 0D 0A 1A 0A`), JPEG (`FF D8 FF`) e WebP (`RIFF....WEBP`), com rejeição estrita a arquivos SVG por motivos de segurança e compatibilidade com PDFs.
- **Motor de Cores OKLCH e Contraste WCAG 2.1**: Implementação pura em TypeScript em `src/lib/theme/color.ts` sem bibliotecas externas, convertendo sRGB $\leftrightarrow$ Linear $\leftrightarrow$ Oklab $\leftrightarrow$ OKLCH, mapeamento de gamut por busca binária e geração de escala semântica em 11 passos (50 a 950), além de cálculo de contraste relativo e seleção automática de `primary-foreground` atendendo ao nível WCAG AA ($\ge 4.5:1$).
- **Conformidade de Tokens CSS**: Criação do script `scripts/check-no-hardcoded-colors.js` integrado ao `pnpm lint`, garantindo que nenhum componente `.svelte` utilize cores literais hardcoded.
- **Congelamento de Migrações**: Migrações `0001` e `0002` foram mantidas intactas, e todas as alterações estruturais do banco foram inseridas na nova migração `0003_branding_dark_logo_and_icon.sql`.

---

## 2. Tabela de Rastreabilidade de Itens e Commits

Todos os itens foram desenvolvidos e commitados individualmente (um commit por item, Conventional Commits em português), com push imediato para o repositório remoto:

| Item    | Descrição Resumida                                                                           |  Commit   |    Status     |
| :------ | :------------------------------------------------------------------------------------------- | :-------: | :-----------: |
| **0.1** | `run_in_tx`: laço de retry cobre `begin_transaction` e `commit` em erros transitórios        | `cef83a4` | **Concluído** |
| **0.2** | Prova com testes de `map_sqlx_err` para `SQLITE_BUSY`, `SQLITE_LOCKED` e permanentes         | `1af787a` | **Concluído** |
| **0.3** | `save_app_config_file` atômico no Windows com `sync_all` e sem remoção prévia                | `c16b7b4` | **Concluído** |
| **0.4** | Reforço da hierarquia impedindo edição de usuários com permissões superiores                 | `10dfbea` | **Concluído** |
| **0.5** | Formatação `Debug` sem segredos para `InitialSetupInput`, `InitialSetupResult`, `UserRecord` | `02bfb37` | **Concluído** |
| **0.6** | Centralização em `CoreServices` com DTO de branding e teste estrutural sem `.stores`         | `c9dd240` | **Concluído** |
| **0.7** | `validate_email` sem `unwrap()`, limite de 64 chars na parte local e suporte a punycode      | `79c535c` | **Concluído** |
| **0.8** | CI com fetch-depth total e validação estrita de congelamento das migrações 0001 e 0002       | `e52c58a` | **Concluído** |
| **0.9** | Persistência de contadores e bloqueio sob falhas em `recover_password`                       | `22de900` | **Concluído** |
| **1.1** | Migração `0003`, suporte a logo escuro e ícone no domínio, store e types                     | `c05db09` | **Concluído** |
| **1.2** | `BrandingValidator` no Core com validação de hex, empresa e magic bytes de imagem            | `e138131` | **Concluído** |
| **1.3** | Motor `color.ts` em TypeScript com OKLCH, escala 50-950 e cálculo WCAG 2.1                   | `5cc47cb` | **Concluído** |
| **1.4** | Escala de variáveis CSS semânticas e linter contra cores literais em `.svelte`               | `b7e7d24` | **Concluído** |
| **1.5** | Tela Configurações > Aparência com seletor OKLCH, preview ao vivo e upload de logos          | `a495e57` | **Concluído** |
| **1.6** | Eliminação de FOUC com cache local síncrono e aplicação de marca no login e header           | `c592e28` | **Concluído** |
| **1.7** | Auditoria transacional resumida sem dados binários ou Data URIs                              | `829baf4` | **Concluído** |
| **1.8** | Testes completos: Vitest de repositório, contrato de store e E2E Playwright                  | `908188f` | **Concluído** |
| **1.9** | Atualização do `docs/04-design-system.md` e `docs/02-modelo-dados.md`                        | `e5e4157` | **Concluído** |

---

## 3. Histórico de Commits (`git log --oneline`)

Histórico linear da branch `fase-1b1-marca` a partir da tag `v0.3.0-fase1b0`:

```text
e5e4157 docs: atualiza design-system e modelo de dados com tokens e colunas de marca
908188f test(branding): adiciona testes e2e playwright e testes de contrato para white label
829baf4 feat(branding): registra auditoria transacional resumida sem dados binarios de imagens
c592e28 feat(ui): elimina fouc de aparencia com cache local e aplica marca em login e header
a495e57 feat(ui): implementa tela de aparencia com seletor oklch preview ao vivo e upload de logos
b7e7d24 feat(theme): aplica escala de tokens css semanticos e adiciona regra contra cores literais
5cc47cb feat(theme): implementa conversao oklch escala de cores e calculo de contraste wcag
e138131 feat(core): implementa brandingvalidator com validacao de cores e magic bytes de imagem
c05db09 feat(db): adiciona migracao 0003 e suporte a logo escuro e icone no dominio e store
22de900 test(auth): valida persistencia de contadores e bloqueio sob falhas em recover_password
e52c58a ci: configura fetch-depth total e ativa validacao estrita de congelamento das migracoes 0001 e 0002
79c535c fix(core): refina validate_email sem unwrap com limite de 64 chars na parte local e suporte a punycode
c9dd240 refactor(tauri): centraliza chamadas em CoreServices com DTO fechado de branding e teste estrutural sem stores
02bfb37 test(core): confirma cobertura completa de formatacao debug sem segredos para todos os tipos sensiveis
10dfbea feat(auth): reforca hierarquia de gestao impedindo edicao de usuarios com permissoes superiores
c16b7b4 fix(desktop): torna gravacao do config.json verdadeiramente atomica com sync_all e sem remocao previa
1af787a test(store-sqlite): valida classificacao de erros transitorios e permanentes em map_sqlx_err
cef83a4 fix(core): inclui begin_transaction e commit no laco de retry de run_in_tx
```

---

## 4. Trechos de Código Principais Solicitados

### 4.1. `run_in_tx` com Retry no `begin_transaction` e `commit` (`crates/core/src/store/mod.rs`)

```rust
pub async fn run_in_tx<T, F>(
    manager: &dyn TransactionManager,
    policy: &TransactionPolicy,
    mut operation: F,
) -> Result<T>
where
    F: FnMut(&dyn TxStoreBundle) -> TxBoxFuture<'_, Result<T>>,
{
    let mut attempt = 0;
    let mut backoff = policy.initial_backoff_ms;

    loop {
        // begin_transaction também entra no laço de retry contra conflitos de concorrência (ex: SQLITE_BUSY no BEGIN IMMEDIATE)
        let tx = match manager.begin_transaction().await {
            Ok(tx) => tx,
            Err(CoreError::TransientConflict(reason)) if attempt < policy.max_retries => {
                attempt += 1;
                tracing::warn!(
                    attempt,
                    backoff_ms = backoff,
                    reason = %reason,
                    "Falha transitória ao abrir transação; tentando novamente após backoff"
                );
                tokio::time::sleep(tokio::time::Duration::from_millis(backoff)).await;
                backoff = (backoff * 2).min(1000);
                continue;
            }
            Err(e) => return Err(e),
        };

        match operation(tx.as_ref()).await {
            Ok(result) => {
                // commit também entra no laço de retry se houver TransientConflict
                match tx.commit().await {
                    Ok(()) => return Ok(result),
                    Err(CoreError::TransientConflict(reason)) if attempt < policy.max_retries => {
                        attempt += 1;
                        tracing::warn!(
                            attempt,
                            backoff_ms = backoff,
                            reason = %reason,
                            "Falha transitória ao confirmar transação; repetindo operação sob nova transação"
                        );
                        tokio::time::sleep(tokio::time::Duration::from_millis(backoff)).await;
                        backoff = (backoff * 2).min(1000);
                        continue;
                    }
                    Err(e) => return Err(e),
                }
            }
            Err(CoreError::TransientConflict(reason)) if attempt < policy.max_retries => {
                attempt += 1;
                let _ = tx.rollback().await;
                tracing::warn!(
                    attempt,
                    backoff_ms = backoff,
                    reason = %reason,
                    "Conflito transitório durante execução da transação; tentando novamente após backoff"
                );
                tokio::time::sleep(tokio::time::Duration::from_millis(backoff)).await;
                backoff = (backoff * 2).min(1000);
            }
            Err(e) => {
                let _ = tx.rollback().await;
                return Err(e);
            }
        }
    }
}
```

### 4.2. `map_sqlx_err` e Classificação de Erros Transitórios (`crates/store-sqlite/src/lib.rs`)

```rust
pub(crate) fn map_sqlx_err(err: sqlx::Error) -> CoreError {
    match err {
        sqlx::Error::Database(ref db_err) => {
            // Códigos SQLite primários e estendidos para SQLITE_BUSY e SQLITE_LOCKED
            let code_str = db_err.code().unwrap_or_default();
            let msg = db_err.message();
            let is_transient = match code_str.as_ref() {
                "5" | "261" | "517" | "773" => true, // SQLITE_BUSY, BUSY_RECOVERY, BUSY_SNAPSHOT, BUSY_TIMEOUT
                "6" | "262" | "518" => true,         // SQLITE_LOCKED, LOCKED_SHAREDCACHE, LOCKED_VTAB
                _ => {
                    let msg_lower = msg.to_ascii_lowercase();
                    msg_lower.contains("database is locked")
                        || msg_lower.contains("database table is locked")
                        || msg_lower.contains("busy")
                }
            };

            if is_transient {
                CoreError::TransientConflict(format!("SQLite ocupado/bloqueado: {msg} (código {code_str})"))
            } else {
                CoreError::Database(err.to_string())
            }
        }
        sqlx::Error::PoolTimedOut => {
            CoreError::TransientConflict("Tempo esgotado ao aguardar conexão livre no pool SQLite".to_string())
        }
        _ => CoreError::Database(err.to_string()),
    }
}
```

### 4.3. `BrandingValidator` no Core (`crates/core/src/domain/branding.rs`)

```rust
pub struct BrandingValidator;

impl BrandingValidator {
    pub const MAX_LOGO_BYTES: usize = 256 * 1024; // 256 KB decodificados
    pub const MAX_ICON_BYTES: usize = 64 * 1024;  // 64 KB decodificados
    pub const MAX_COMPANY_NAME_LEN: usize = 80;

    pub fn validate_company_name(name: &str) -> Result<String> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(CoreError::Validation("Nome da empresa não pode ser vazio.".to_string()));
        }
        if trimmed.chars().count() > Self::MAX_COMPANY_NAME_LEN {
            return Err(CoreError::Validation(format!(
                "Nome da empresa excede o limite máximo de {} caracteres.",
                Self::MAX_COMPANY_NAME_LEN
            )));
        }
        if trimmed.chars().any(|c| c.is_control()) {
            return Err(CoreError::Validation(
                "Nome da empresa não pode conter caracteres de controle.".to_string(),
            ));
        }
        Ok(trimmed.to_string())
    }

    pub fn validate_hex_color(field_name: &str, color: &str) -> Result<String> {
        let trimmed = color.trim();
        if !trimmed.starts_with('#') || trimmed.len() != 7 {
            return Err(CoreError::Validation(format!(
                "Cor {field_name} inválida '{trimmed}'. Use o formato hexadecimal #RRGGBB."
            )));
        }
        if !trimmed[1..].chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(CoreError::Validation(format!(
                "Cor {field_name} contém caracteres não-hexadecimais '{trimmed}'. Use o formato hexadecimal #RRGGBB."
            )));
        }
        Ok(trimmed.to_ascii_lowercase())
    }

    pub fn validate_image_data_uri(field_name: &str, data_uri: &str, is_icon: bool) -> Result<String> {
        let trimmed = data_uri.trim();

        // Rejeição estrita a SVG por segurança
        let lower = trimmed.to_ascii_lowercase();
        if lower.contains("<svg") || lower.contains("image/svg") {
            return Err(CoreError::Validation(
                "Imagens no formato SVG não são permitidas por motivos de segurança. Utilize PNG, JPEG ou WebP."
                    .to_string(),
            ));
        }

        if !trimmed.starts_with("data:image/") {
            return Err(CoreError::Validation(format!(
                "{field_name} deve ser um Data URI iniciando com 'data:image/'."
            )));
        }

        let prefix_end = trimmed.find(";base64,").ok_or_else(|| {
            CoreError::Validation(format!(
                "{field_name} deve ser codificado em base64 no formato 'data:image/<tipo>;base64,<dados>'."
            ))
        })?;

        let mime = &trimmed[5..prefix_end].trim().to_ascii_lowercase();
        let normalized_mime = match mime.as_str() {
            "image/png" => "image/png",
            "image/jpeg" | "image/jpg" => "image/jpeg",
            "image/webp" => "image/webp",
            other => {
                return Err(CoreError::Validation(format!(
                    "Formato de imagem '{other}' não suportado para {field_name}. Utilize PNG, JPEG ou WebP."
                )));
            }
        };

        let payload = &trimmed[prefix_end + 8..];
        let decoded = decode_base64(payload).ok_or_else(|| {
            CoreError::Validation(format!("Dados de base64 inválidos ou corrompidos para {field_name}."))
        })?;

        let max_size = if is_icon { Self::MAX_ICON_BYTES } else { Self::MAX_LOGO_BYTES };
        if decoded.len() > max_size {
            return Err(CoreError::Validation(format!(
                "{field_name} excede o limite máximo permitido de {} KB (tamanho decodificado: {} KB).",
                max_size / 1024,
                (decoded.len() + 1023) / 1024
            )));
        }

        // Validação estrita de magic bytes de acordo com o MIME declarado
        match normalized_mime {
            "image/png" => {
                const PNG_MAGIC: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
                if decoded.len() < 8 || &decoded[0..8] != &PNG_MAGIC {
                    return Err(CoreError::Validation(format!(
                        "Assinatura mágica do arquivo PNG inválida ou corrompida para {field_name}."
                    )));
                }
            }
            "image/jpeg" => {
                const JPEG_MAGIC: [u8; 3] = [0xFF, 0xD8, 0xFF];
                if decoded.len() < 3 || &decoded[0..3] != &JPEG_MAGIC {
                    return Err(CoreError::Validation(format!(
                        "Assinatura mágica do arquivo JPEG inválida ou corrompida para {field_name}."
                    )));
                }
            }
            "image/webp" => {
                if decoded.len() < 12 || &decoded[0..4] != b"RIFF" || &decoded[8..12] != b"WEBP" {
                    return Err(CoreError::Validation(format!(
                        "Assinatura mágica do arquivo WebP inválida ou corrompida para {field_name}."
                    )));
                }
            }
            _ => unreachable!(),
        }

        Ok(format!("data:{normalized_mime};base64,{payload}"))
    }
}
```

### 4.4. Escala OKLCH e Contraste WCAG 2.1 (`src/lib/theme/color.ts`)

```typescript
export function oklchToGamutMappedRgb(
  l: number,
  c: number,
  h: number,
): [number, number, number] {
  if (l <= 0) return [0, 0, 0];
  if (l >= 1) return [1, 1, 1];

  let raw = oklchToLinearRgbInternal(l, c, h);
  let srgb = [
    linearToSrgb(raw[0]),
    linearToSrgb(raw[1]),
    linearToSrgb(raw[2]),
  ] as [number, number, number];

  if (inGamut(srgb)) {
    return [
      Math.min(1, Math.max(0, srgb[0])),
      Math.min(1, Math.max(0, srgb[1])),
      Math.min(1, Math.max(0, srgb[2])),
    ];
  }

  // Busca binária rápida no chroma para ajustar no gamut sRGB sem alteração de matiz ou brilho
  let low = 0;
  let high = c;
  let bestRgb: [number, number, number] = [0, 0, 0];

  for (let i = 0; i < 20; i++) {
    const mid = (low + high) / 2;
    raw = oklchToLinearRgbInternal(l, mid, h);
    srgb = [linearToSrgb(raw[0]), linearToSrgb(raw[1]), linearToSrgb(raw[2])];
    if (inGamut(srgb)) {
      bestRgb = [
        Math.min(1, Math.max(0, srgb[0])),
        Math.min(1, Math.max(0, srgb[1])),
        Math.min(1, Math.max(0, srgb[2])),
      ];
      low = mid;
    } else {
      high = mid;
    }
  }

  return bestRgb;
}

export function generateColorScale(hex: string): ColorScale {
  const base = hexToOklch(hex);
  const scale = {} as ColorScale;

  for (const step of SCALE_STEPS) {
    const config = SCALE_CONFIG[step];
    const targetL = config.l;
    const targetC = base.c * config.cFactor;
    const targetH = base.h;

    const mappedHex = oklchToHex(targetL, targetC, targetH);
    const css = oklchToCss(targetL, targetC, targetH);

    scale[step] = {
      hex: mappedHex,
      oklch: css,
      l: targetL,
      c: targetC,
      h: targetH,
    };
  }

  return scale;
}

export function getAccessibleForeground(
  backgroundHex: string,
): ForegroundOption {
  const whiteHex = "#ffffff";
  const darkHex = "#090d16";
  const whiteCss = "oklch(0.99 0 0)";
  const darkCss = "oklch(0.12 0.015 260)";

  const contrastWhite = calculateContrast(backgroundHex, whiteHex);
  const contrastDark = calculateContrast(backgroundHex, darkHex);

  if (contrastWhite >= contrastDark) {
    return {
      hex: whiteHex,
      css: whiteCss,
      contrast: contrastWhite,
      isAccessible: contrastWhite >= 4.5,
    };
  } else {
    return {
      hex: darkHex,
      css: darkCss,
      contrast: contrastDark,
      isAccessible: contrastDark >= 4.5,
    };
  }
}
```

### 4.5. Passo de CI de Congelamento das Migrações 0001 e 0002 (`.github/workflows/ci.yml`)

```yaml
- name: Validar congelamento das migrações 0001 e 0002
  shell: bash
  run: |
    git diff --name-only origin/main...HEAD | { grep -E '^crates/store-sqlite/migrations/000[12]_' || true; } > changed_migrations.txt
    if [ -s changed_migrations.txt ]; then
      echo "::error::As migrações 0001 e 0002 estão CONGELADAS. Modificações detectadas:"
      cat changed_migrations.txt
      exit 1
    else
      echo "Verificação concluída: migrações 0001 e 0002 mantidas intactas."
    fi
```

---

## 5. Resultados de Validação

- **Workspace Rust**:
  - `cargo test --workspace`: 40 testes aprovados (8 app lib, 12 core, 20 store-sqlite), 0 falhas.
  - `cargo check --workspace`: compilação limpa sem warnings.
- **Frontend & TypeScript**:
  - `svelte-check --tsconfig ./tsconfig.json`: 0 erros e 0 warnings.
  - `pnpm lint`: linter de cores literais, Prettier e ESLint 100% aprovados.
  - `vitest run`: 18 testes unitários/integração aprovados.
  - `playwright test`: 2 testes E2E aprovados (`smoke.spec.ts` e `branding.spec.ts`).
