# 02 - Modelo de Dados: EventRent

## 1. Convenções de Entidades e Tabelas
Toda tabela no EventRent segue rigorosamente as regras estruturais:
- `id TEXT PRIMARY KEY`: Identificador único UUID v7 ordenável no tempo (em formato texto canônico).
- `tenant_id TEXT NOT NULL`: Identificador do tenant proprietário dos dados (para isolamento multi-tenant desde o início).
- `created_at TEXT NOT NULL`: Timestamp UTC da criação em ISO-8601 (`YYYY-MM-DDTHH:MM:SSZ`).
- `updated_at TEXT NOT NULL`: Timestamp UTC da última modificação em ISO-8601.
- `deleted_at TEXT`: Timestamp UTC de exclusão lógica (soft delete). Nulo para registros ativos.
- `created_by TEXT`: Identificador do usuário criador (quando houver autenticação).

## 2. Tabelas da Fundação (Fase 0)

### 2.1. `tenant`
Armazena a identificação das empresas clientes (mesmo em modo desktop local, há ao menos o tenant default).

| Coluna | Tipo | Restrições | Descrição |
|---|---|---|---|
| `id` | TEXT | PRIMARY KEY | UUID v7 |
| `name` | TEXT | NOT NULL | Razão social ou nome fantasia |
| `slug` | TEXT | NOT NULL UNIQUE | Identificador curto para rotas e domínio |
| `document_number` | TEXT | | CNPJ ou documento fiscal |
| `created_at` | TEXT | NOT NULL | Data de criação UTC |
| `updated_at` | TEXT | NOT NULL | Data de atualização UTC |
| `deleted_at` | TEXT | | Soft delete |

### 2.2. `tenant_branding`
Configurações de identidade visual e personalização para a empresa locadora.

| Coluna | Tipo | Restrições | Descrição |
|---|---|---|---|
| `id` | TEXT | PRIMARY KEY | UUID v7 |
| `tenant_id` | TEXT | NOT NULL UNIQUE, FK(tenant.id) | Tenant associado |
| `company_name` | TEXT | NOT NULL | Nome de exibição no app e documentos |
| `primary_color` | TEXT | NOT NULL | Cor primária (código hex/oklch) |
| `secondary_color`| TEXT | NOT NULL | Cor secundária |
| `accent_color` | TEXT | NOT NULL | Cor de destaque |
| `logo_url` | TEXT | | Caminho relativo, URI local ou data URL do logo |
| `favicon_url` | TEXT | | Ícone do tenant |
| `created_at` | TEXT | NOT NULL | Data de criação UTC |
| `updated_at` | TEXT | NOT NULL | Data de atualização UTC |
| `deleted_at` | TEXT | | Soft delete |

## 3. Diretrizes de Tipos de Dados
- **Monetário**: Valores financeiros sempre em **inteiro** (centavos de Real, ex.: R$ 1.500,00 -> `150000`). Jamais armazenar ponto flutuante (`float`/`double`).
- **Datas e Horas**: Strings no formato ISO-8601 UTC.
- **Campos JSON**: Quando permitidos, serializados como texto estruturado e validados por esquemas de domínio.
