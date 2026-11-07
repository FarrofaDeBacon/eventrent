# Modelo de Segurança e Limites do Modo Local — EventRent

Este documento descreve com transparência o modelo de segurança, o modelo de ameaças e os limites técnicos do EventRent operando no **Modo Local (Desktop-first com SQLite)**.

---

## 1. Limite Honesto do Modo Local (SQLite sem Criptografia em Repouso)

No Modo Local, os dados do aplicativo são gravados em um arquivo SQLite padrão (`eventrent.db`), localizado no diretório de dados do usuário (`app_data_dir`).

> [!WARNING]
> **O banco de dados SQLite local NÃO é criptografado em repouso.**
> O sistema operacional tem acesso de leitura direta ao arquivo `.db`. Qualquer usuário ou processo com privilégios de leitura no sistema de arquivos local pode abrir o banco de dados diretamente utilizando utilitários como `sqlite3`, DBeaver ou extensões do VS Code, sem passar pelas camadas de autenticação da aplicação.

---

## 2. O que a Autenticação e o RBAC Protegem

A camada de segurança da aplicação (composta pelo runtime Tauri, traits do Core, Argon2id e controle de acesso RBAC) fornece as seguintes garantias:

1. **Controle de Acesso na Interface (Aplicação)**:
   - Impede que operadores não autenticados utilizem as funções da aplicação.
   - Força o logout automático após período de inatividade configurável (padrão: 30 minutos).

2. **Isolamento de Papéis e Privilégios (RBAC)**:
   - Todos os comandos não públicos passam obrigatoriamente pela guarda central (`require_session_and_permission`).
   - Um usuário com perfil _Comercial_, _Estoque_, _Técnico_ ou _Financeiro_ não consegue criar usuários, alterar perfis de sistema ou modificar configurações administrativas da empresa via interface.

3. **Proteção Contra Força Bruta Online**:
   - Políticas progressivas de bloqueio temporário (_lockout_ exponencial):
     - 1 a 4 falhas: aviso de credenciais inválidas em tempo constante.
     - 5 falhas consecutivas: bloqueio temporário de 5 minutos.
     - 6 falhas consecutivas: 10 minutos.
     - 7 falhas consecutivas: 20 minutos.
     - 8+ falhas: teto de 60 minutos de bloqueio.
   - A mesma política de bloqueio progressivo é aplicada estritamente à **Chave de Recuperação**, persistida nas colunas `recovery_failed_attempts` e `recovery_locked_until` da tabela `user`.

4. **Proteção Criptográfica de Senhas e Chaves**:
   - Hashes gerados com **Argon2id** (versão 0x13) em conformidade com as recomendações de ponta do **OWASP Password Storage Cheat Sheet**:
     - Memória ($m$): 19 MiB ($19.456$ KiB);
     - Iterações de tempo ($t$): 2 passes;
     - Grau de paralelismo ($p$): 1 thread;
     - Salt criptográfico aleatório por hash via `OsRng`.
   - Execução estritamente desacoplada da thread assíncrona do Tokio via `tokio::task::spawn_blocking`, impedindo qualquer congelamento de interface ou degradação do event loop.

5. **Chave de Recuperação de Alta Entropia com Rotação Obrigatória**:
   - Formato: `ER-XXXX-XXXX-XXXX-XXXX` (16 símbolos aleatórios divididos em 4 blocos).
   - Alfabeto: 32 caracteres seguros (`ABCDEFGHJKLMNPQRSTUVWXYZ23456789`), excluindo intencionalmente os caracteres visualmente ambíguos `0`, `O`, `1` e `I`.
   - Entropia matemática: $\log_2(32^{16}) = 16 \times 5 = 80$ bits de entropia criptográfica pura.
   - Rotação atômica: a cada recuperação de senha bem-sucedida, a chave utilizada é **imediatamente invalidada** e uma **nova chave de recuperação** é gerada, exibida uma única vez ao usuário e armazenada exclusivamente na forma de hash Argon2id.

6. **Auditoria Imutável (Append-Only)**:
   - Todas as operações críticas (`auth.login_success`, `auth.login_failed`, `auth.account_locked`, `auth.recovery_failed`, `auth.recovery_locked`, `user.password_recovered`, `auth.recovery_key_rotated`, `user.created`, `user.updated`, `branding.updated`) são registradas na tabela `audit_log` para fins de compliance e suporte.

---

## 3. O que a Autenticação Local NÃO Protege (Fora do Escopo Local)

- **Acesso Físico / Administrador Local (Root/Admin)**: Um usuário com privilégios de administrador na máquina local pode ler o arquivo `eventrent.db` ou extraí-lo do disco.
- **Malware no Host**: Programas maliciosos executando na mesma sessão de usuário do sistema operacional podem inspecionar a memória do processo ou o arquivo SQLite.
- **Backups não criptografados**: Se o usuário copiar o arquivo `eventrent.db` para um pendrive ou pasta pública, os dados estarão legíveis.

---

## 4. Recomendações aos Usuários no Modo Local

Enquanto a aplicação opera no modo local:

1. Utilizar a criptografia de disco nativa do sistema operacional (**BitLocker** no Windows, **FileVault** no macOS, **LUKS** no Linux).
2. Manter contas de usuário separadas no sistema operacional caso múltiplos funcionários usem o mesmo computador físico.
3. Guardar a Chave de Recuperação impressa ou em cofre de senhas corporativo seguro fora do próprio computador.

---

## 6. Vetor de Negação de Serviço por Bloqueio de Conta e Requisitos da Fase 2.5 (Nuvem)

### 6.1 Análise de Ameaça: Negação de Serviço (DoS) via Bloqueio de Conta

O bloqueio de conta baseado exclusivamente em e-mail (`failed_attempts` e `locked_until`) introduz um vetor clássico de negação de serviço:

- Um ator mal-intencionado (ou concorrente) que conheça o e-mail do Proprietário pode disparar propositalmente 5 tentativas incorretas de login, forçando o bloqueio da conta por 5 a 60 minutos e impedindo o acesso do usuário legítimo às operações da empresa.
- No **Modo Local Desktop**, essa ameaça tem impacto mitigado porque o software roda em um computador físico controlado e a Chave de Recuperação permite redefinir a senha e zerar o bloqueio imediatamente. Por essa razão, a aplicação desktop local adota a política `LoginPolicy { reveal_account_locked: true }` para fornecer feedback claro ao operador.

### 6.2 Política de Login Desacoplada (`LoginPolicy`)

O núcleo do sistema (`crates/core`) adota como padrão estrito `reveal_account_locked: false`:

- Quando `reveal_account_locked` é falso (recomendado para ambientes de rede/nuvem), tentativas de login contra uma conta temporariamente bloqueada **não revelam** o bloqueio. Em vez disso, o sistema executa a carga criptográfica completa de Argon2id contra a constante `DUMMY_ARGON2_HASH` e devolve genericamente `CoreError::InvalidCredentials`.
- Essa política elimina qualquer canal lateral de enumeração de contas e reduz a atratividade de ataques de DoS por e-mail.

### 6.3 Requisitos Mandatórios para a Fase 2.5 (Servidor Axum e PostgreSQL)

Para a transição da nuvem na Fase 2.5, as seguintes salvaguardas serão obrigatórias:

1. **Limitação de Taxa por IP e Dispositivo (Rate Limiting)**: O bloqueio e o throttling serão aplicados por endereço IP de origem e identificadores de dispositivo, evitando que requisições vindas de atacantes bloqueiem o acesso do usuário legítimo em seu IP corporativo/residencial.
2. **Confiança Zero no Cliente**: O backend HTTP jamais aceitará estruturas `SessionInfo` originadas do frontend. A autenticação utilizará tokens de sessão opacos gerados e armazenados no servidor (ou cookies seguros `HttpOnly`, `SameSite=Strict`), com validação rigorosa de assinatura e revogação no banco.
3. **Mecanismo de Desbloqueio e Alertas**: Disparo assíncrono de notificações transacionais por e-mail quando tentativas anômalas forem detectadas, permitindo desbloqueio seguro por link assinado sem depender exclusivamente do tempo de expiração.
