# Relatório de Conclusão: Fase 1A.2 — Endurecimento

**Data**: 06/11/2026  
**Branch**: `fase-1a2-hardening`  
**Pull Request**: [#3](https://github.com/FarrofaDeBacon/eventrent/pull/3)  
**Status**: Concluído — Pronto para Revisão e Aprovação

---

## 1. Resumo Executivo

A Fase 1A.2 teve como foco exclusivo o endurecimento arquitetural, de segurança e de confiabilidade do sistema, sem a introdução de funcionalidades novas. Foram abordados todos os apontamentos identificados na auditoria pós-Fase 1A, preparando a base de código tanto para a operação desktop local (Tauri 2 + SQLite) quanto para a futura reutilização do `crates/core` no servidor HTTP Axum com PostgreSQL (Fase 2.5 e Fase 3).

Esta foi a última rodada antes do congelamento definitivo das migrações `0001_initial_schema.sql` e `0002_users_roles_and_audit.sql`.

---

## 2. Entregas Realizadas

### Grupo A: Testes e Guarda de Comandos
- **A1. Relógio Único de Sessão**: Eliminada a função duplicada em `guards.rs`. O núcleo de domínio (`crates/core`) agora expõe `verify_session_active_at(&self, now: DateTime<Utc>)`, utilizada diretamente pela produção via `verify_session_active()`. Os testes unitários agora exercitam exatamente a lógica de produção, passando timestamps simulados e validando os limites precisos de 15 min (ativo), 30 min (limite exato) e 31 min (expirado).
- **A2. Registro e Integridade Estrutural de Comandos**:
  - Implementado teste de contrato estrutural (`test_command_registry_matches_generate_handler_in_lib_rs`) que extrai programaticamente os comandos declarados no macro `tauri::generate_handler!` em `src-tauri/src/lib.rs` e valida paridade 1:1 com o `COMMAND_REGISTRY`.
  - As permissões exigidas em `COMMAND_REGISTRY` foram unificadas com o core (`users:read` para listagem de usuários e consulta de perfis; `users:manage` para criação e alteração).
  - Testes garantem que qualquer comando não listado na whitelist pública obrigatoriamente passe pela guarda e exija sessão autenticada.

### Grupo B: Segurança no Core
- **B1. Login Sem Enumeração de Usuários**: Quando um usuário não é encontrado ou está inativo, o serviço de autenticação executa `verify_secret` contra uma constante fictícia padronizada de Argon2id OWASP (`DUMMY_ARGON2_HASH`). Isso uniformiza a curva de tempo de resposta e previne ataques de timing e enumeração de credenciais.
- **B2. Contadores de Falha Atômicos com `RETURNING`**: Novas operações de store (`increment_login_failure` e `increment_recovery_failure`) realizam o incremento atômico diretamente na instrução SQL `UPDATE` retornando o valor atualizado (`RETURNING failed_attempts`), eliminando janelas de corrida concorrentes.
- **B3. Validação Estrita de Senha e E-mail**:
  - Senha: Entre 8 e 128 caracteres contados em Unicode (`password.chars().count()`).
  - E-mail: Validação RFC rigorosa sem adição de novas dependências (tamanho total de 5 a 254 caracteres, presença de arroba única, validação de partes local e de domínio, sem pontos repetidos ou periféricos).
- **B4. Anti-Escalada de Privilégios**: Em `create_user` e `update_user`, o serviço valida se o conjunto de permissões do perfil a ser atribuído é um subconjunto estrito das permissões que o próprio usuário da sessão possui.
- **B5. Comparação de Conjuntos em `ensure_system_roles`**: A garantia idempotente de perfis agora compara coleções (`HashSet<&str>`) de permissões em vez de contagem numérica. Caso haja qualquer divergência, os perfis são atualizados e uma entrada informativa é persistida em `audit_log`.
- **B6. Debug Seguro sem Vazamento de Segredos**: Implementação customizada de `std::fmt::Debug` para todos os DTOs sensíveis (`LoginInput`, `CreateUserInput`, `UpdateUserInput`, `ResetPasswordInput`, `RecoverPasswordInput`, `InitialSetupInput`, `UserRecord`, `InitialSetupResult`, `RecoverPasswordResult`). Teste automatizado garante que a saída de log nunca contenha senhas ou chaves de recuperação.
- **B7. Reset Administrativo Completo**: A alteração administrativa de senha agora limpa integralmente `failed_attempts` e remove o bloqueio temporário `locked_until`.
- **B8. Documentação Técnica da Chave de Recuperação**: Ajustado o comentário criptográfico da chave de recuperação para refletir a especificação técnica exata (80 bits de entropia CSPRNG com hashing Argon2id OWASP).

### Grupo C: Banco de Dados e Compatibilidade PostgreSQL
- **C1. Renomeação para `app_user`**: Renomeada a tabela `user` para `app_user` em todo o código e na migração `0002_users_roles_and_audit.sql`, prevenindo colisões com palavras reservadas em dialetos SQL padrão (especialmente PostgreSQL).
- **C2. Trilha de Auditoria Append-Only**: Removidas as colunas `updated_at` e `deleted_at` da tabela `audit_log`. A trait `AuditStore` não expõe métodos de mutação ou deleção, com teste de contrato atestando sua imutabilidade.
- **C3. Índice em Relacionamento**: Adicionado índice `idx_app_user_role_id` em `app_user(role_id)`.
- **C4. Mapeamento PostgreSQL no ADR-0007**: Registradas as diretrizes e diferenças arquiteturais para a Fase 2.5 (tipos booleanos nativos, timestamps com fuso horário, placeholders posicionais).

### Grupo D: Aplicação Desktop e CI
- **D1. Branding Público Desacoplado**: Criado o comando `get_public_branding` (e método `get_public_branding` no repositório frontend), permitindo que telas pré-autenticação (`/setup` e `/login`) exibam a identidade visual (nome, cores e logo) da empresa sem demandar token de sessão.
- **D2. Resiliência contra Mutex Envenenado**: O acesso ao estado global da aplicação (`AppState`) utiliza `lock().unwrap_or_else(PoisonError::into_inner)`, evitando falhas silenciosas ou travamento do desktop em caso de pânico anterior.
- **D3. Gravação Atômica de Configuração**: O arquivo `config.json` agora é gravado utilizando um arquivo temporário intermediário (`config.json.tmp.<ts>`) seguido por renomeação atômica (`fs::rename`), prevenindo corrupção em quedas de energia. Teste unitário incluído.
- **D4. Otimização e Timeout no CI**:
  - Definido `timeout-minutes: 20` para todos os jobs no workflow de CI (`.github/workflows/ci.yml`).
  - Adicionado cache específico para os binários de navegadores do Playwright em `~/.cache/ms-playwright` no Linux.
  - Causa do tempo elevado anterior (46m): A corrida inicial em um runner frio realizou instalação redundante de pacotes de sistema, download completo dos navegadores do Playwright e compilação do zero de todas as crates do workspace sem sccache/target quente. Com o cache de cargo e pnpm já populado, o tempo médio estabilizou em ~4 minutos.
- **D5. Correção de Roadmap**: Conforme documentado em `docs/05-roadmap.md`, a próxima fase é oficialmente a **Fase 1B (Clientes e Marca Completa)**; Catálogo de Equipamentos e Estoque pertence à **Fase 2**.

---

## 3. ADR-0008: Proposta de Transações nas Stores

Foi redigido o documento de decisão arquitetural `docs/decisions/0008-transacoes-nas-stores.md`, detalhando três abordagens para atomicidade multi-store (Unit of Work com `TransactionManager`, Callback Transacional e Operações Compostas) e estabelecendo a recomendação formal para avaliação da Direção Técnica antes de qualquer implementação.

---

## 4. Próximos Passos

Aguardando a revisão deste relatório, validação do ADR-0008 e autorização para o merge do PR #3, seguido pela liberação da **Fase 1B (Clientes e Marca Completa)**.
