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

---

## 7. Privacidade e Conformidade LGPD no Módulo de Clientes (Fase 1B.2)

O tratamento de dados pessoais e fiscais de clientes (CPF, CNPJ, contatos e endereços) observa o princípio de minimização em trilhas de auditoria e telemetria:

1. **Mascaramento de Dados Pessoais em Auditoria e Logs (`mask_document_for_audit`, `mask_email_for_audit`, `mask_phone_for_audit`)**:
   - Os registros em `audit_log` referentes a criação, atualização, ativação/desativação e exclusão lógica de clientes (`customer.created`, `customer.updated`, `customer.activated`, `customer.deactivated`, `customer.deleted`) não expõem CPF, CNPJ, e-mail ou telefone completos:
     - **CPF**: mascarado preservando apenas os 2 dígitos verificadores (`***.***.***-25`).
     - **CNPJ**: mascarado preservando apenas o identificador de ordem/filial (`**.***.***/0001-**`).
     - **E-mail**: mascarado preservando inicial e domínio (`j***@dominio.com`).
     - **Telefone**: mascarado preservando DDD e os 4 últimos dígitos (`(21) *****-1234`).
   - Todas as structs de domínio de cliente (`Customer`, `CustomerContact`, `CustomerAddress`, inputs e `CustomerFilter`) implementam `fmt::Debug` manual com ofuscação/mascaramento, impedindo vazamento acidental em `{:?}`.
2. **Guarda de Comandos Tauri**:
   - Os 5 comandos de clientes (`list_customers`, `get_customer`, `create_customer`, `update_customer`, `delete_customer`) passam obrigatoriamente por `require_session(&state)` no backend Rust, validam expiração por inatividade e delegam integralmente às permissões `customer:read` / `customer:manage` em `CoreServices`, sem acesso direto das interfaces aos repositórios de banco.
3. **Ocultação de Erros Cru de Banco (`map_sqlx_err`)**:
   - Conflitos de unicidade de documento (código SQLite `2067` / `1555`) são convertidos em mensagens de domínio amigáveis (`CoreError::Conflict`) sem vazar consultas SQL ou estrutura interna de índices ao usuário.
4. **Trilha de Auditoria Append-Only e Ausência de PII Direta**:
   - A tabela `audit_log` é estritamente **append-only** (protegida por triggers SQLite contra `UPDATE` e `DELETE`).
   - Por ser imutável e permanente, a trilha de auditoria **não pode conter PII direta/crua** (nomes completos de pessoas físicas, e-mails ou telefones não mascarados, números de documentos completos).
   - Todos os eventos em `audit_log` passam por funções de mascaramento criptográfico e de privacidade no Core:
     - Nomes de titulares pessoa física: `mask_individual_name_for_audit` (ex.: `J*** S***`);
     - E-mails de usuários e contatos: `mask_email_for_audit` (ex.: `u***@dominio.com`);
     - Telefones de contato: `mask_phone_for_audit` (ex.: `(21) *****-1234`);
     - Documentos fiscais (CPF/CNPJ): `mask_document_for_audit`.
   - Fica registrada expressamente a **pendência da Política de Retenção e Descarte de Logs (Art. 16 da LGPD)** para implementação na Fase 2.5/Fase 7, definindo prazos de expiração (ex.: 5 anos para compliance contábil/fiscal) e rotina segura de expurgo ou arquivamento frio de logs históricos.

5. **Pendência de Anonimização LGPD e `search_text`**:
   - A coluna derivada `customer.search_text` armazena cópia normalizada de dados pessoais (nome, documento, e-mail e telefone) para viabilizar busca textual rápida.
   - Fica registrada como requisito obrigatório de conformidade LGPD a futura operação de **anonimização de titular** (ex.: mediante solicitação de exclusão definitiva quando houver retenção fiscal/contratual), a qual deverá ofuscar `name`, `trade_name`, `document_number`, contatos, endereços e **limpar integralmente `search_text`**, preservando apenas os vínculos financeiros/fiscais anônimos.

---

## 8. Segregação de Dados Financeiros de Fornecedores (Fase 1C)

Para manter o princípio do menor privilégio e evitar exposição desnecessária a perfis operacionais (Estoque e Comercial):

- **Dados sensíveis fora do cadastro unificado nesta fase:** **Chave PIX**, **dados bancários** (banco, agência, conta, titularidade) e **condições de pagamento** **NÃO são armazenados** nas tabelas `customer`, `customer_contact`, `customer_address` ou `customer_supplier_category` na Fase 1C.
- **Planejamento para a Fase 6 (Financeiro):** Essas informações serão modeladas em tabela/domínio financeiro dedicado na **Fase 6**, protegidas por permissões financeiras restritas (`finance:read` e `finance:manage`), mascaramento/criptografia adequada e auditoria específica, impedindo acesso por perfis sem atribuição financeira.

---

## 9. Segurança de Mídia, Rejeição de EXIF, Isolamento de PDF e Backup (Fase 2M / ADR-0013)

1. **Endereçamento por Conteúdo (SHA-256) e Contenção de Caminho (`FsMediaStore`)**:
   - Nenhum caminho de arquivo no disco é derivado de nomes enviados pelo usuário. A chave de armazenamento é derivada exclusivamente de `<tenant_id>/<2_primeiros_hex_sha256>/<sha256_64_hex>` dentro de `<dados_do_app>/media/`, com verificação estrita de contenção de diretório e gravação atômica (`arquivo temporário` + `sync_all` + `rename`).
   - O nome original do arquivo (`original_filename`) é saneado no Core (`sanitize_media_filename`), removendo prefixos de caminho, rejeitando `..` e caracteres de controle, e limitando-se a 150 caracteres.
2. **Validação de Magic Bytes, Limites Pré-Processamento e Rejeição de EXIF**:
   - A transferência binária ocorre pelo canal raw (`tauri::ipc::Request` / `tauri::ipc::Response`) do Tauri v2 sem Base64 gigante, com verificação de tamanho máximo aplicada no Rust **antes** de alocar ou processar o payload (`600 KB` para exibição, `60 KB` para miniatura, `25 MB` para PDF).
   - O Core valida a coerência entre os magic bytes do cabeçalho e o `mime_type` declarado (`image/webp`, `image/jpeg`, `image/png`, `application/pdf`) e lê as dimensões reais do cabeçalho via `imagesize` (ADR-0010).
   - **Privacidade e Proteção contra Vazamento de Geolocalização (EXIF)**: O frontend redimensiona e reencoda imagens via `<canvas>` (WebP com fallback JPEG) antes do envio — eliminando metadados de câmera e GPS — e o Core inspeciona os chunks binários e **rejeita obrigatoriamente** qualquer imagem que ainda contenha metadados EXIF (WebP chunk `EXIF` ou bit EXIF no `VP8X`, JPEG `APP1 Exif`, PNG chunk `eXIf`).
3. **Proibição de Download por URL (Prevenção de SSRF) e Isolamento de PDFs**:
   - O download automático de imagens ou documentos a partir de URLs externas é **estritamente proibido** na primeira versão para eliminar vetores de _Server-Side Request Forgery_ (SSRF).
   - **Documentos PDF nunca são interpretados nem renderizados dentro do WebView do aplicativo**. A visualização grava uma cópia temporária com nome saneado em `<dados_do_app>/temp-media/<uuid>/<nome_saneado>` e a delega ao leitor nativo do sistema operacional (`tauri-plugin-opener`), ou exporta via diálogo nativo `"Salvar como..."` (`tauri-plugin-dialog`). A pasta `temp-media` é limpa integralmente ao iniciar e ao encerrar o aplicativo.
4. **Exibição Segura via Blob URLs (Sem Protocolo `asset://` Amplo) e Auditoria Enxuta**:
   - O protocolo de assets do Tauri com escopo amplo no sistema de arquivos não é utilizado. O frontend solicita os bytes via comando autenticado (`get_media_blob_bytes`, validado por `tenant_id` e `catalog:read`) e gerencia `Blob URL`s em um cache LRU em memória com limites de itens e bytes, revogando `URL.revokeObjectURL` automaticamente na evicção.
   - A trilha de auditoria (`audit_log`) registra o `sha256`, tamanho em bytes, dimensões e nome saneado na mesma transação, **jamais armazenando os bytes binários** do arquivo.
5. **Backup, Migração para Nuvem (Fase 2.6) e Antivírus na Nuvem (Fase 2.5)**:
   - Os blobs da árvore `<dados_do_app>/media/` integram obrigatoriamente o pacote de backup local e o assistente de migração local para nuvem (Fase 2.6), com verificação de integridade SHA-256 ponta a ponta.
   - Na operação em nuvem (Fase 2.5), todo blob enviado passará por verificação assíncrona de antivírus/antimalware em quarentena antes de ser disponibilizado para download pelos usuários do tenant.
