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
   - Um usuário com perfil *Comercial*, *Estoque*, *Técnico* ou *Financeiro* não consegue criar usuários, alterar perfis de sistema ou modificar configurações administrativas da empresa via interface.

3. **Proteção Contra Força Bruta Online**:
   - Políticas progressivas de bloqueio temporário (*lockout* exponencial):
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

## 5. Roadmap: Criptografia em Repouso (SQLCipher)

A implementação de criptografia transparente em repouso no banco SQLite foi mapeada como item de arquitetura para decisões futuras:
- **Pendência Registrada**: Avaliação de integração com **SQLCipher** (via `sqlx` com feature de encriptação ou biblioteca sidecar).
- **Desafios a serem resolvidos**:
  - Gerenciamento seguro da chave mestra de decodificação no cliente desktop (armazenamento em TPM / Windows Credential Manager / macOS Keychain / Linux Secret Service);
  - Complexidade de compilação estática em múltiplos sistemas operacionais (cross-compilation no CI);
  - Migração de dados entre o modo local SQLite puro e o futuro modo Nuvem PostgreSQL (Fase 2.5).
