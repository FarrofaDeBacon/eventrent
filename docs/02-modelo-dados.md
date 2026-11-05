# 02 - Modelo de Dados: EventRent

## 1. Convenções de Entidades e Tabelas

Toda tabela no EventRent segue rigorosamente as regras estruturais:

- `id TEXT PRIMARY KEY`: Identificador único UUID v7 ordenável no tempo (em formato texto canônico).
- `tenant_id TEXT NOT NULL`: Identificador do tenant proprietário dos dados (para isolamento multi-tenant desde o início).
- `created_at TEXT NOT NULL`: Timestamp UTC da criação em ISO-8601 (`YYYY-MM-DDTHH:MM:SSZ`).
- `updated_at TEXT NOT NULL`: Timestamp UTC da última modificação em ISO-8601.
- `deleted_at TEXT`: Timestamp UTC de exclusão lógica (soft delete). Nulo para registros ativos.
- `created_by TEXT`: Identificador do usuário criador (quando aplicável).
- **Índices Únicos Parciais**: Nenhuma tabela com exclusão lógica (`deleted_at`) utiliza `UNIQUE` em linha na definição da coluna. Toda restrição de unicidade lógica é implementada via índice único parcial (`CREATE UNIQUE INDEX ... WHERE deleted_at IS NULL`), compatível com SQLite e PostgreSQL, permitindo reaproveitar slugs e e-mails após exclusão lógica.

## 2. Configuração Pré-Banco (`config.json`)

Antes da abertura de qualquer banco de dados, o aplicativo desktop lê `<app_data_dir>/config.json`:

| Campo                     | Tipo    | Descrição                                                        |
| ------------------------- | ------- | ---------------------------------------------------------------- |
| `mode`                    | TEXT    | Modo de operação (`"local"` ou `"cloud"`).                       |
| `setup_completed`         | BOOLEAN | Indica se o assistente de primeira execução foi concluído.       |
| `session_timeout_minutes` | INTEGER | Tempo limite de inatividade da sessão em minutos (padrão: `30`). |
| `cloud_server_url`        | TEXT?   | URL da API remota (utilizado apenas quando `mode = "cloud"`).    |

## 3. Tabelas da Fundação (`0001_init_tenant_and_branding.sql`)

### 3.1. `tenant`

Armazena a identificação das empresas locadoras (em modo local há o tenant padrão `default`).

| Coluna            | Tipo | Restrições  | Descrição                                |
| ----------------- | ---- | ----------- | ---------------------------------------- |
| `id`              | TEXT | PRIMARY KEY | UUID v7                                  |
| `name`            | TEXT | NOT NULL    | Razão social ou nome fantasia            |
| `slug`            | TEXT | NOT NULL    | Identificador curto para rotas e domínio |
| `document_number` | TEXT |             | CNPJ ou documento fiscal                 |
| `created_at`      | TEXT | NOT NULL    | Data de criação UTC                      |
| `updated_at`      | TEXT | NOT NULL    | Data de atualização UTC                  |
| `deleted_at`      | TEXT |             | Soft delete                              |
| `created_by`      | TEXT |             | Usuário criador                          |

- **Índice único parcial**: `idx_tenant_slug_active ON tenant(slug) WHERE deleted_at IS NULL`

### 3.2. `tenant_branding`

Configurações de identidade visual e personalização da empresa locadora.

| Coluna            | Tipo | Restrições              | Descrição                                         |
| ----------------- | ---- | ----------------------- | ------------------------------------------------- |
| `id`              | TEXT | PRIMARY KEY             | UUID v7                                           |
| `tenant_id`       | TEXT | NOT NULL, FK(tenant.id) | Tenant associado                                  |
| `company_name`    | TEXT | NOT NULL                | Nome de exibição no app (padrão: `Minha empresa`) |
| `primary_color`   | TEXT | NOT NULL                | Cor primária                                      |
| `secondary_color` | TEXT | NOT NULL                | Cor secundária                                    |
| `accent_color`    | TEXT | NOT NULL                | Cor de destaque                                   |
| `logo_url`        | TEXT |                         | Caminho relativo, URI local ou data URL do logo   |
| `favicon_url`     | TEXT |                         | Ícone do tenant                                   |
| `created_at`      | TEXT | NOT NULL                | Data de criação UTC                               |
| `updated_at`      | TEXT | NOT NULL                | Data de atualização UTC                           |
| `deleted_at`      | TEXT |                         | Soft delete                                       |
| `created_by`      | TEXT |                         | Usuário criador                                   |

- **Índice único parcial**: `idx_tenant_branding_tenant_active ON tenant_branding(tenant_id) WHERE deleted_at IS NULL`

## 4. Tabelas de Usuários, Perfis e Auditoria (`0002_users_roles_and_audit.sql`)

### 4.1. `role`

Perfis de acesso por tenant (os 5 perfis iniciais são marcados com `is_system = 1`).

| Coluna        | Tipo    | Restrições              | Descrição                                            |
| ------------- | ------- | ----------------------- | ---------------------------------------------------- |
| `id`          | TEXT    | PRIMARY KEY             | UUID v7                                              |
| `tenant_id`   | TEXT    | NOT NULL, FK(tenant.id) | Tenant associado                                     |
| `name`        | TEXT    | NOT NULL                | Nome de exibição (`Proprietário`, `Comercial`, etc.) |
| `slug`        | TEXT    | NOT NULL                | Slug canônico (`proprietario`, `comercial`, etc.)    |
| `description` | TEXT    | NOT NULL                | Descrição das responsabilidades do perfil            |
| `is_system`   | INTEGER | NOT NULL DEFAULT 0      | `1` para perfis nativos protegidos contra exclusão   |
| `created_at`  | TEXT    | NOT NULL                | Data de criação UTC                                  |
| `updated_at`  | TEXT    | NOT NULL                | Data de atualização UTC                              |
| `deleted_at`  | TEXT    |                         | Soft delete                                          |
| `created_by`  | TEXT    |                         | Usuário criador                                      |

- **Índice único parcial**: `idx_role_tenant_slug_active ON role(tenant_id, slug) WHERE deleted_at IS NULL`

### 4.2. `role_permission`

Associação entre perfis (`role`) e constantes de permissão (`resource:action`).

| Coluna           | Tipo | Restrições              | Descrição                              |
| ---------------- | ---- | ----------------------- | -------------------------------------- |
| `id`             | TEXT | PRIMARY KEY             | UUID v7                                |
| `tenant_id`      | TEXT | NOT NULL, FK(tenant.id) | Tenant associado                       |
| `role_id`        | TEXT | NOT NULL, FK(role.id)   | Perfil associado                       |
| `permission_key` | TEXT | NOT NULL                | Chave de permissão (`resource:action`) |
| `created_at`     | TEXT | NOT NULL                | Data de criação UTC                    |
| `updated_at`     | TEXT | NOT NULL                | Data de atualização UTC                |
| `deleted_at`     | TEXT |                         | Soft delete                            |
| `created_by`     | TEXT |                         | Usuário criador                        |

- **Índice único parcial**: `idx_role_permission_unique_active ON role_permission(tenant_id, role_id, permission_key) WHERE deleted_at IS NULL`

### 4.3. `user`

Contas de usuários pertencentes a um tenant, com credenciais Argon2id e controle de bloqueio temporário.

| Coluna                  | Tipo    | Restrições              | Descrição                                                         |
| ----------------------- | ------- | ----------------------- | ----------------------------------------------------------------- |
| `id`                    | TEXT    | PRIMARY KEY             | UUID v7                                                           |
| `tenant_id`             | TEXT    | NOT NULL, FK(tenant.id) | Tenant associado                                                  |
| `role_id`               | TEXT    | NOT NULL, FK(role.id)   | Perfil de acesso atribuído                                        |
| `name`                  | TEXT    | NOT NULL                | Nome completo do usuário                                          |
| `email`                 | TEXT    | NOT NULL                | E-mail normalizado em minúsculas                                  |
| `password_hash`         | TEXT    | NOT NULL                | Hash Argon2id da senha                                            |
| `recovery_key_hash`     | TEXT    |                         | Hash Argon2id da chave de recuperação (apenas para Proprietários) |
| `is_active`             | INTEGER | NOT NULL DEFAULT 1      | `1` ativo, `0` desativado                                         |
| `failed_login_attempts` | INTEGER | NOT NULL DEFAULT 0      | Contador de falhas consecutivas de autenticação                   |
| `locked_until`          | TEXT    |                         | Timestamp ISO-8601 UTC até quando o login está bloqueado          |
| `last_login_at`         | TEXT    |                         | Timestamp ISO-8601 UTC do último login bem-sucedido               |
| `created_at`            | TEXT    | NOT NULL                | Data de criação UTC                                               |
| `updated_at`            | TEXT    | NOT NULL                | Data de atualização UTC                                           |
| `deleted_at`            | TEXT    |                         | Soft delete                                                       |
| `created_by`            | TEXT    |                         | Usuário criador                                                   |

- **Índice único parcial**: `idx_user_tenant_email_active ON user(tenant_id, email) WHERE deleted_at IS NULL`

### 4.4. `audit_log`

Trilha imutável de auditoria para operações sensíveis de segurança, usuários, perfis e configurações.

| Coluna        | Tipo | Restrições              | Descrição                                                          |
| ------------- | ---- | ----------------------- | ------------------------------------------------------------------ |
| `id`          | TEXT | PRIMARY KEY             | UUID v7                                                            |
| `tenant_id`   | TEXT | NOT NULL, FK(tenant.id) | Tenant associado                                                   |
| `entity_type` | TEXT | NOT NULL                | Tipo da entidade afetada (`user`, `role`, `tenant_branding`, etc.) |
| `entity_id`   | TEXT | NOT NULL                | ID do registro afetado                                             |
| `action`      | TEXT | NOT NULL                | Ação executada (`user.created`, `auth.login_succeeded`, etc.)      |
| `actor_id`    | TEXT |                         | ID do usuário que realizou a ação                                  |
| `before_json` | TEXT |                         | Estado anterior serializado em JSON (sem hashes de senha)          |
| `after_json`  | TEXT |                         | Novo estado serializado em JSON (sem hashes de senha)              |
| `created_at`  | TEXT | NOT NULL                | Data da ocorrência UTC                                             |
| `updated_at`  | TEXT | NOT NULL                | Data de atualização UTC                                            |
| `deleted_at`  | TEXT |                         | Reservado para conformidade estrutural                             |
| `created_by`  | TEXT |                         | ID do autor do evento                                              |

## 5. Diretrizes de Tipos de Dados

- **Monetário**: Valores financeiros sempre em **inteiro** (centavos de Real, ex.: R$ 1.500,00 -> `150000`). Jamais armazenar ponto flutuante (`float`/`double`).
- **Datas e Horas**: Strings no formato ISO-8601 UTC.
- **Campos JSON**: Quando permitidos, serializados como texto estruturado e validados por esquemas de domínio, nunca contendo segredos em texto puro ou hashes de autenticação.
