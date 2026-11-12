# 03 - Regras de Negócio: EventRent

## 1. Princípios Gerais de Negócio

As regras de negócio do EventRent operam de maneira determinística, implementadas nos serviços de domínio de `crates/core` (`CoreServices`), sem depender do mecanismo de armazenamento concreto (`store-sqlite` ou futuro `store-postgres`).

## 2. Modos de Operação e Assistente de Primeira Execução

### 2.1. Dois Modos de Operação (`ADR-0006`)

- O modo de operação é armazenado em `<app_data_dir>/config.json` **fora do banco de dados**, pois deve ser lido antes mesmo de saber se o aplicativo abrirá o SQLite local ou conectará a uma API em nuvem:
  - **Modo Local (`local`)**: Um computador, banco SQLite local em `<app_data_dir>/eventrent.db`, operação 100% offline. Ativo desde a Fase 1A.
  - **Modo Nuvem (`cloud`)**: Múltiplos computadores compartilhando a mesma empresa via servidor central (Fastify + PostgreSQL). Visível no assistente de instalação com indicação _"Em breve"_, habilitado na Fase 2.5.

### 2.2. Primeira Execução e Chave de Recuperação

- Quando `setup_completed = false` (ou quando não existe nenhum usuário ativo no tenant padrão), o sistema redireciona obrigatoriamente para o assistente `/setup`.
- No modo **Local**, o assistente solicita:
  1. Nome da empresa locadora (substituindo o nome padrão `"Minha empresa"`).
  2. Nome completo do Proprietário.
  3. E-mail de login do Proprietário.
  4. Senha inicial (mínimo de 8 caracteres).
- Ao concluir o cadastro inicial, o sistema gera uma **Chave de Recuperação** única no formato `ER-XXXX-XXXX-XXXX-XXXX` (alfabeto sem caracteres ambíguos), exibe-a uma única vez ao Proprietário com exigência de confirmação de leitura/guarda, e grava no banco apenas seu hash `Argon2id`.
- Como o modo local opera sem servidor de e-mail, a Chave de Recuperação permite ao Proprietário redefinir sua senha diretamente na tela de login (`/login`), zerando também eventuais bloqueios temporários da conta.

## 3. Autenticação, Segurança e Sessão

### 3.1. Hashes de Senha e Segredos

- Todas as senhas e chaves de recuperação são processadas exclusivamente com **Argon2id** e salt criptográfico aleatório (`OsRng`).
- **Regra de Segurança**: Jamais registrar senhas em texto puro, chaves de recuperação em texto puro ou hashes de senha em arquivos de log (`tracing`) ou nos payloads `before_json`/`after_json` da tabela `audit_log`.

### 3.2. Bloqueio Progressivo por Tentativas Falhas

- A cada falha de senha no login, o contador `failed_login_attempts` do usuário é incrementado:
  - **1 a 4 falhas**: Sem bloqueio temporal; tentativa registrada.
  - **5ª falha consecutiva**: Conta bloqueada por **5 minutos** (`300s`) e evento `auth.account_locked` gravado no `audit_log`.
  - **6ª falha consecutiva**: Conta bloqueada por **10 minutos** (`600s`).
  - **7ª falha consecutiva**: Conta bloqueada por **20 minutos** (`1200s`).
  - **8ª falha ou mais**: Conta bloqueada por **60 minutos** (`3600s`).
- Um login bem-sucedido ou uma redefinição de senha válida zera `failed_login_attempts` e limpa `locked_until`.

### 3.3. Sessão e Expiração por Inatividade

- A sessão autenticada é mantida no estado do processo Rust (`AppState`) com `last_activity_at` e `timeout_minutes` (configurável em `config.json`, padrão de `30` minutos).
- Toda chamada autenticada verifica se a sessão expirou por inatividade; se não expirou, renova `last_activity_at` (`touch_session`).

## 4. Controle de Acesso Baseado em Perfis e Permissões (RBAC)

### 4.1. Catálogo de Permissões (`resource:action`)

As permissões são definidas como constantes imutáveis em `crates/core/src/domain/permission.rs`:

| Constante              | Chave (`resource:action`) | Descrição                                                   |
| ---------------------- | ------------------------- | ----------------------------------------------------------- |
| `DASHBOARD_VIEW`       | `dashboard:view`          | Visualizar painel operacional e indicadores gerais          |
| `USERS_VIEW`           | `users:view`              | Listar e consultar usuários do tenant                       |
| `USERS_CREATE`         | `users:create`            | Cadastrar novos usuários                                    |
| `USERS_EDIT`           | `users:edit`              | Editar nome, e-mail e perfil de usuários                    |
| `USERS_DEACTIVATE`     | `users:deactivate`        | Ativar ou desativar contas de usuários                      |
| `USERS_RESET_PASSWORD` | `users:reset_password`    | Redefinir senha de outros usuários                          |
| `ROLES_VIEW`           | `roles:view`              | Consultar perfis de acesso e matriz de permissões           |
| `ROLES_MANAGE`         | `roles:manage`            | Gerenciar perfis personalizados (fases futuras)             |
| `BRANDING_VIEW`        | `branding:view`           | Consultar configurações de aparência e marca                |
| `BRANDING_EDIT`        | `branding:edit`           | Alterar nome da empresa, cores e logotipo                   |
| `SETTINGS_VIEW`        | `settings:view`           | Acessar área de configurações do sistema                    |
| `SETTINGS_EDIT`        | `settings:edit`           | Alterar parâmetros globais do sistema                       |
| `AUDIT_VIEW`           | `audit:view`              | Consultar trilha de auditoria (`audit_log`)                 |
| `INVENTORY_VIEW`       | `inventory:view`          | Consultar catálogo de equipamentos, números de série e kits |
| `INVENTORY_EDIT`       | `inventory:edit`          | Cadastrar e editar equipamentos, categorias e estoque       |
| `QUOTES_VIEW`          | `quotes:view`             | Consultar clientes, orçamentos e contratos                  |
| `QUOTES_EDIT`          | `quotes:edit`             | Criar, editar e aprovar orçamentos e reservas               |
| `OPERATIONS_VIEW`      | `operations:view`         | Consultar ordens de separação, saída, retorno e manutenção  |
| `OPERATIONS_EXECUTE`   | `operations:execute`      | Executar conferência de saída/retorno e abrir manutenção    |
| `FINANCE_VIEW`         | `finance:view`            | Consultar contas a receber, faturamento e fluxo de caixa    |
| `FINANCE_EDIT`         | `finance:edit`            | Registrar baixas, cobranças e ajustes financeiros           |

### 4.2. Perfis de Sistema Padrão

Criados de forma idempotente por `ensure_system_roles` para cada tenant:

1. **Proprietário (`proprietario`)**: Possui **todas as 21 permissões** do sistema.
2. **Comercial (`comercial`)**: `dashboard:view`, `branding:view`, `inventory:view`, `quotes:view`, `quotes:edit`, `finance:view`.
3. **Estoque (`estoque`)**: `dashboard:view`, `branding:view`, `inventory:view`, `inventory:edit`, `operations:view`, `operations:execute`.
4. **Técnico (`tecnico`)**: `dashboard:view`, `branding:view`, `inventory:view`, `operations:view`, `operations:execute`.
5. **Financeiro (`financeiro`)**: `dashboard:view`, `branding:view`, `quotes:view`, `finance:view`, `finance:edit`.

### 4.3. Regras de Proteção de Contas e Auditoria

1. **Verificação no Core**: Toda operação em `CoreServices` exige `SessionInfo` válida e valida explicitamente a permissão `resource:action` antes de executar qualquer leitura ou escrita sensível.
2. **Proteção do Último Proprietário**: É proibido desativar (`set_user_active(..., false)`) ou alterar o perfil (`update_user`) de um usuário `Proprietário` caso ele seja o último `Proprietário` ativo do tenant (`count_active_owners <= 1`).
3. **Proibição de Autodesativação**: Um usuário nunca pode desativar a própria conta (`actor.user_id == target_user_id`).
4. **Auditoria Obrigatória (`audit_log`)**: Toda criação/edição/ativação/desativação de usuário, redefinição de senha, recuperação por chave, bloqueio de conta e alteração de marca grava um registro em `audit_log` com `actor_id`, `action`, `before_json` e `after_json`.

## 5. Regras Fundamentais do Domínio de Locação

### 5.1. Controle Híbrido de Estoque (Fase 1B)

- Cada modelo de equipamento (`equipment_model`) possui a propriedade `control_mode`:
  - `serialized`: Controle individualizado por número de série e patrimônio. Cada item possui histórico de manutenção, horas de uso e estado físico.
  - `quantity`: Controle por quantidade e saldo disponível em estoque. Indicado para cabos, adaptadores, ganchos e consumíveis.
- **Kits**: Agrupamentos lógicos de equipamentos (ex.: Kit Iluminação Palco com 8 refletores + mesa DMX + cabeamento). O kit expande em seus componentes na fase de separação e reserva.

### 5.2. Motor de Disponibilidade e Reserva (Fase 3)

- A verificação de disponibilidade e a efetivação da reserva de equipamentos ocorrem obrigatoriamente na **mesma transação de banco de dados**.
- Não é permitido split-brain ou conferência prévia desvinculada da gravação, eliminando integralmente condições de corrida (_race conditions_) e _overbooking_.
- O cálculo de disponibilidade considera períodos de trânsito, montagem, evento, desmontagem e margem de segurança para manutenção.

### 5.3. Identidade Visual e White Label

- O nome inicial padrão da empresa antes da configuração pelo cliente é `"Minha empresa"`, evitando exibir a marca do software (`EventRent`) como se fosse a razão social da locadora.
- A paleta de cores do tenant sobrescreve dinamicamente as variáveis de tema no `:root` sem que nenhum componente precise de código condicional.
- O tema respeita alternância entre modo claro e modo escuro, assegurando que o contraste atenda aos critérios WCAG AA.

### 5.4. Cadastro de Clientes, Documentos e Instrução Normativa RFB nº 2.229, de 15 de outubro de 2024 (Fase 1B.2)

- **Tipos de Pessoa e Documentos**:
  - **Pessoa Física (`individual`)**: aceita `cpf` (11 dígitos numéricos, Módulo 11, rejeitando sequências repetidas), `foreign` (passaporte/documento estrangeiro: 5 a 30 caracteres alfanuméricos normalizados em maiúsculas, sem dígito verificador) ou `none` (pré-cadastro sem documento).
  - **Pessoa Jurídica (`legal`)**: aceita `cnpj` (14 posições, numérico tradicional ou alfanumérico conforme **Instrução Normativa RFB nº 2.229, de 15 de outubro de 2024**), `foreign` ou `none`.
  - **Restrição para Contratos e Notas Fiscais (Fases 4 e 7)**: Embora o cadastro inicial permita `document_type = 'none'` (para pré-cadastro/orçamento rápido) ou `foreign`, a **emissão de contratos (Fase 4) e notas fiscais (Fase 7) EXIGEM obrigatoriamente `cpf` ou `cnpj` válidos**.
- **Regra do CNPJ Alfanumérico (Instrução Normativa RFB nº 2.229, de 15 de outubro de 2024)**:
  - As 12 primeiras posições aceitam caracteres alfanuméricos `[0-9A-Z]` (convertidos por `ASCII - 48`: `'0'..'9' -> 0..9` e `'A'..'Z' -> 17..42`).
  - Os 2 últimos caracteres são estritamente dígitos verificadores numéricos `[0-9]`, calculados em Módulo 11 com pesos `[5,4,3,2,9,8,7,6,5,4,3,2]` e `[6,5,4,3,2,9,8,7,6,5,4,3,2]`.
- **Unicidade e Soft Delete**:
  - O documento normalizado é único entre clientes ativos do mesmo tenant via índice parcial (`idx_customer_tenant_document_active WHERE deleted_at IS NULL`). Ao excluir logicamente um cliente, o documento fica liberado para eventual recadastro futuro sem violar histórico.
- **Integridade de Contatos e Endereços**:
  - Todo cliente deve possuir no mínimo **1 contato** e **1 endereço** cadastrados, mantendo ao menos um registro sinalizado como principal (`is_primary`).
- **Busca Desacentuada e Paginação Determinística**:
  - Os campos normalizados `search_text` e `name_sort` removem acentuação e padronizam caixa baixa para busca instantânea em SQLite e paginação por cursor composto `(name_sort, id)` sem duplicações ou lacunas entre páginas.
- **Exclusão Lógica (`delete_customer`) e Ativação (`set_active`)**:
  - `delete_customer` executa **exclusão lógica** (`soft delete`), preenchendo `deleted_at` em `customer`, `customer_contact` e `customer_address` na mesma transação e gravando evento `customer.deleted` em `audit_log`, liberando o documento no índice único parcial para eventual recadastro.
  - `set_active` (permissão `customer:manage`) alterna a flag `active` (`true`/`false`) preservando integralmente o cadastro e gravando `customer.activated` / `customer.deactivated` no `audit_log`.
  - **Regra Obrigatória da Fase 4 (Orçamentos e Contratos)**: Quando existirem orçamentos, reservas ou contratos vinculados ao cliente, a exclusão (`delete_customer`) **deverá ser bloqueada** pelo domínio, orientando o usuário a utilizar a desativação (`set_active(false)`).
- **Autorização (RBAC)**:
  - Consulta permitida com `customer:read` (**Proprietário**, **Comercial**, **Financeiro**).
  - Criação, edição e exclusão lógica exigem `customer:manage` (**Proprietário**, **Comercial**).
