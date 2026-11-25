# 03 - Regras de Negócio: EventRent

## 1. Princípios Gerais de Negócio

As regras de negócio do EventRent operam de maneira determinística, implementadas nos serviços de domínio de `crates/core` (`CoreServices`), sem depender do mecanismo de armazenamento concreto (`store-sqlite` ou futuro `store-postgres`).

## 2. Modos de Operação e Assistente de Primeira Execução

### 2.1. Dois Modos de Operação (`ADR-0006`)

- O modo de operação é armazenado em `<app_data_dir>/config.json` **fora do banco de dados**, pois deve ser lido antes mesmo de saber se o aplicativo abrirá o SQLite local ou conectará a uma API em nuvem:
  - **Modo Local (`local`)**: Um computador, banco SQLite local em `<app_data_dir>/eventrent.db`, operação 100% offline. Ativo desde a Fase 1A.
  - **Modo Nuvem (`cloud`)**: Múltiplos computadores compartilhando a mesma empresa via servidor central (Axum + PostgreSQL). Visível no assistente de instalação com indicação _"Em breve"_, habilitado na Fase 2.5.

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

### 4.1. Catálogo de Permissões

As permissões são definidas como constantes imutáveis em `crates/core/src/domain/permission.rs` (`ALL_PERMISSIONS`):

| Constante                 | Chave                | Descrição                                                   |
| ------------------------- | -------------------- | ----------------------------------------------------------- |
| `PERM_USER_READ`          | `user:read`          | Listar e consultar usuários do tenant                       |
| `PERM_USER_MANAGE`        | `user:manage`        | Criar, editar, ativar, desativar e redefinir senha          |
| `PERM_ROLE_READ`          | `role:read`          | Consultar perfis de acesso e matriz de permissões           |
| `PERM_ROLE_MANAGE`        | `role:manage`        | Criar e gerenciar perfis personalizados                     |
| `PERM_BRANDING_READ`      | `branding:read`      | Consultar configurações de aparência e marca                |
| `PERM_BRANDING_MANAGE`    | `branding:manage`    | Alterar nome da empresa, cores, logotipo e ícone            |
| `PERM_CUSTOMER_READ`      | `customer:read`      | Consultar clientes e contatos                               |
| `PERM_CUSTOMER_MANAGE`    | `customer:manage`    | Criar e editar clientes e contatos                          |
| `PERM_SUPPLIER_READ`      | `supplier:read`      | Consultar fornecedores                                      |
| `PERM_SUPPLIER_MANAGE`    | `supplier:manage`    | Criar e editar fornecedores                                 |
| `PERM_VENUE_READ`         | `venue:read`         | Consultar locais e espaços de eventos                       |
| `PERM_VENUE_MANAGE`       | `venue:manage`       | Criar e editar locais e espaços de eventos                  |
| `PERM_CATALOG_READ`       | `catalog:read`       | Consultar catálogo de modelos, marcas e categorias          |
| `PERM_CATALOG_MANAGE`     | `catalog:manage`     | Criar e editar modelos, fotos, manuais, marcas e categorias |
| `PERM_RESERVATION_READ`   | `reservation:read`   | Consultar reservas de equipamentos                          |
| `PERM_RESERVATION_MANAGE` | `reservation:manage` | Criar, alterar e cancelar reservas de equipamentos          |
| `PERM_QUOTE_READ`         | `quote:read`         | Consultar orçamentos e propostas comerciais                 |
| `PERM_QUOTE_MANAGE`       | `quote:manage`       | Criar, editar e aprovar orçamentos e propostas comerciais   |
| `PERM_WAREHOUSE_READ`     | `warehouse:read`     | Consultar ordens de separação, saída, retorno e estoque     |
| `PERM_WAREHOUSE_OPERATE`  | `warehouse:operate`  | Executar conferência de saída/retorno e manutenção          |
| `PERM_FINANCE_READ`       | `finance:read`       | Consultar contas a receber, faturamento e fluxo de caixa    |
| `PERM_FINANCE_MANAGE`     | `finance:manage`     | Registrar baixas, cobranças e ajustes financeiros           |
| `PERM_AUDIT_READ`         | `audit:read`         | Consultar trilha de auditoria (`audit_log`)                 |
| `PERM_SETTINGS_READ`      | `settings:read`      | Consultar parâmetros globais do sistema                     |
| `PERM_SETTINGS_MANAGE`    | `settings:manage`    | Alterar parâmetros globais do sistema                       |
| `PERM_BACKUP_MANAGE`      | `backup:manage`      | Gerenciar backups e restaurar estado do sistema             |

### 4.2. Perfis de Sistema Padrão

Criados de forma idempotente por `ensure_system_roles` para cada tenant:

1. **Proprietário (`owner`)**: Possui **todas as 26 permissões** do catálogo canônico (`ALL_PERMISSIONS`).
2. **Comercial (`commercial`)**: `customer:read`, `customer:manage`, `supplier:read`, `venue:read`, `venue:manage`, `catalog:read`, `reservation:read`, `reservation:manage`, `quote:read`, `quote:manage`, `branding:read`.
3. **Estoque (`warehouse`)**: `supplier:read`, `supplier:manage`, `venue:read`, `venue:manage`, `catalog:read`, `catalog:manage`, `reservation:read`, `warehouse:read`, `warehouse:operate`, `branding:read`.
4. **Técnico (`technician`)**: `venue:read`, `catalog:read`, `reservation:read`, `warehouse:read`, `warehouse:operate`, `branding:read`.
5. **Financeiro (`finance`)**: `customer:read`, `supplier:read`, `supplier:manage`, `venue:read`, `quote:read`, `reservation:read`, `finance:read`, `finance:manage`, `audit:read`, `branding:read`.

### 4.3. Regras de Proteção de Contas e Auditoria

1. **Verificação no Core**: Toda operação em `CoreServices` exige `SessionInfo` válida e valida explicitamente as permissões canônicas antes de executar qualquer leitura ou escrita sensível.
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
  - **Contato Obrigatório (mínimo 1)**: Todo cliente deve possuir pelo menos 1 contato com telefone OU e-mail válidos, com exatamente 1 contato principal (`is_primary`). Limite máximo de 20 contatos.
  - **Endereço Opcional (zero ou mais)**: O cadastro inicial não exige endereço (zero ou mais endereços cadastrados), viabilizando pré-cadastros rápidos e orçamentos ágeis de locação (onde o local de entrega é definido por evento na Fase 4). Se informados endereços, deve haver exatamente 1 principal (`is_primary`). Limite máximo de 20 endereços.
  - **Exigência Fiscal (Fase 7)**: O endereço completo passa a ser estritamente **obrigatório no momento da emissão de notas fiscais (Fase 7)**.
- **Busca Desacentuada e Paginação Determinística**:
  - Os campos normalizados `search_text` e `name_sort` removem acentuação (formas NFC e marcas combinantes NFD `U+0300..U+036F`) e padronizam caixa baixa para busca em SQLite e paginação por cursor composto `(name_sort, id)`.
  - **Limite de Escala da Busca por Trecho (`LIKE '%termo%'`)**: Na Fase 1B.2, a consulta `WHERE tenant_id = ? AND deleted_at IS NULL AND search_text LIKE ? ORDER BY name_sort ASC, id ASC` utiliza o índice `idx_customer_tenant_sort (tenant_id=?)` e realiza varredura linear sobre os registros ativos do tenant (~11 ms para 20.000 registros). Para volumes muito maiores de clientes por tenant, a evolução planejada para a **Fase 2.5** utilizará **FTS5** no SQLite local e **índices trigram (`pg_trgm` com GIN/GiST)** no PostgreSQL.
- **Exclusão Lógica (`delete_customer`) e Ativação (`set_active`)**:
  - `delete_customer` executa **exclusão lógica** (`soft delete`), preenchendo `deleted_at` em `customer`, `customer_contact` e `customer_address` na mesma transação e gravando evento `customer.deleted` em `audit_log`, liberando o documento no índice único parcial para eventual recadastro.
  - `set_active` (permissão `customer:manage`) alterna a flag `active` (`true`/`false`) preservando integralmente o cadastro e gravando `customer.activated` / `customer.deactivated` no `audit_log`.
  - **Regra Obrigatória da Fase 4 (Orçamentos e Contratos)**: Quando existirem orçamentos, reservas ou contratos vinculados ao cliente, a exclusão (`delete_customer`) **deverá ser bloqueada** pelo domínio, orientando o usuário a utilizar a desativação (`set_active(false)`).
- **Autorização (RBAC)**:
  - Consulta permitida com `customer:read` (**Proprietário**, **Comercial**, **Financeiro**).
  - Criação, edição e exclusão lógica exigem `customer:manage` (**Proprietário**, **Comercial**).
- **Telefones em E.164 e Seletor de DDI**:
  - Todo telefone é normalizado e armazenado em formato **E.164 com prefixo `+`** (`+55...` para Brasil; `+DDI...` com 8 a 15 dígitos para outros países).
  - No `core`, qualquer telefone informado **sem `+`** é tratado como número brasileiro (ganha `+55` e exige 10 ou 11 dígitos). Por isso, o frontend disponibiliza seletor de DDI (padrão `+55`) em todos os campos de telefone e envia números de outros países sempre em E.164 com `+`.

## 6. Locais de Evento, Espaços e Configurações de Alocação (Fase 1D)

- **Locais e Espaços**:
  - Um local (`venue`) pode ter até **100 espaços** (`venue_space`) e **20 contatos** (`venue_contact`).
  - Nomes de espaços são únicos dentro do mesmo local (comparação normalizada sem acentos e sem distinção de maiúsculas).
  - Na atualização de um local (`update_venue`), o backend preserva os `id`s estáveis dos espaços existentes informados pelo cliente (rejeitando `id` desconhecido ou pertencente a outro local).
  - **Regra Obrigatória da Fase 4 (Eventos e Contratos)**: Espaços já referenciados por eventos/orçamentos/contratos **não poderão ser excluídos** na atualização do local nem o local poderá sofrer exclusão lógica; o usuário deverá utilizar a desativação (`active = false`).
  - Duplicidade de `nome + cidade` dentro do mesmo tenant gera **aviso não bloqueante** na interface para permitir múltiplas unidades quando necessário.
- **Tempos Operacionais e Configurações do Tenant (`tenant_setting`)**:
  - Chaves padrão: `allocation.changeover_minutes` (virada no mesmo espaço, padrão `30`, faixa `0..=240`), `allocation.internal_transfer_minutes` (deslocamento entre espaços do mesmo local, padrão `20`, faixa `0..=240`) e `allocation.external_transfer_minutes` (deslocamento entre locais diferentes, padrão `60`, faixa `0..=720`).
  - `venue.internal_transfer_minutes` é opcional (`0..=240` minutos): `NULL` utiliza o padrão da empresa (`allocation.internal_transfer_minutes`) e `0` explícito representa zero.
  - Leitura exige `settings:read` e alteração exige `settings:manage` com registro em `audit_log`.
- **Autorização (RBAC) de Locais**:
  - `venue:read`: **Proprietário**, **Comercial**, **Estoque**, **Técnico**, **Financeiro**.
  - `venue:manage`: **Proprietário**, **Comercial**, **Estoque**.

---

### Catálogo de Equipamentos, Categorias e Atributos (Fase 2A)

- **Hierarquia e Ciclos**: Árvore de categorias limitada a **4 níveis** de profundidade, com bloqueio de auto-referência e ciclos. Exclusão bloqueada se houver subcategorias ou modelos vinculados.
- **Herança de Campos Técnicos**: Modelos herdam automaticamente os atributos configurados em todas as categorias ancestrais até sua categoria. Máximo de **50 atributos** por categoria.
- **Validações Técnicas e Fiscais**:
  - `ean`: aceita 8, 12, 13 ou 14 dígitos numéricos com validação obrigatória do dígito verificador GS1.
  - `ncm`: exatamente 8 dígitos numéricos.
  - Pesos, dimensões e corrente positivos (`> 0`); potência e valores monetários em centavos não negativos (`>= 0`).
- **Carga Sugerida Idempotente**: Ação explícita no serviço/UI (`load_suggested_catalog_categories`) cria árvore audiovisual inicial sem duplicar registros em execuções repetidas.
- **Matriz de Permissões do Catálogo**:
  - `catalog:read`: **Proprietário**, **Estoque**, **Comercial**, **Técnico**.
  - `catalog:manage`: **Proprietário**, **Estoque**.
  - **Financeiro**: sem acesso ao catálogo.

---

### Endurecimento do Catálogo (Tarefa 0) e Mídia de Modelos (Fase 2M / ADR-0013)

- **Proteção de Definições de Atributos e Chaves Efetivas**:
  - É proibido alterar `data_type` de um atributo técnico quando já existem modelos com valores preenchidos (erro `Conflict` informando quantos modelos utilizam o campo), bem como remover opções de lista (`select`) que estejam em uso (erro `Conflict` listando as opções em uso; adicionar novas opções é livre).
  - `field_key` é único em toda a cadeia efetiva (ancestrais e descendentes) ao criar/editar atributo ou ao mover uma subárvore de categoria.
- **Sincronização Transacional de Busca e Marcas/Categorias Inativas**:
  - Renomear marca ou renomear/mover categoria recalcula o `search_text` de todos os modelos afetados na **mesma transação**.
  - Marcas ou categorias inativas não podem ser atribuídas a modelos novos nem ao trocar a marca/categoria de um modelo existente (modelos que já as utilizavam permanecem válidos ao editar outros campos).
  - `list_models` aplica limite padrão `50` (quando ausente ou `0`) e teto máximo `100`, escapando `%`, `_` e `\` com `ESCAPE '\'`.
  - Pré-checagem explícita de duplicidade de `SKU` e `(marca + código do modelo)` informa o nome do modelo que já utiliza o valor.
  - A carga sugerida idempotente inclui as raízes **"DJ"** (`Players (CDJ)`, `Mixers DJ`, `Controladoras DJ`) e **"Acessórios"** (`Pedestais e suportes`, `Cases e racks`, `Fitas e consumíveis`).
- **Mídia de Modelos (Fotos e Documentos Técnicos PDF)**:
  - **Limites por Modelo**: no máximo **12 fotos** e **20 documentos PDF** por modelo de equipamento.
  - **Validação Binária no Core**:
    - Imagens (`image/webp`, `image/jpeg`, `image/png`) validadas por magic bytes coerentes com o MIME declarado, dimensões lidas via `imagesize` (exibição até **2048 px** no maior lado e **600 KB**; miniatura até **512 px** e **60 KB**) e **rejeição obrigatória de metadados EXIF** (WebP chunk `EXIF` / flag `VP8X`, JPEG `APP1 Exif`, PNG chunk `eXIf`).
    - Documentos PDF (`application/pdf`) validados por assinatura `%PDF-` e limite de **25 MB**.
    - Metadados: `title` de `1..=150` caracteres, `credit_note` e `version_note` até `300` caracteres, e `original_filename` saneado (sem diretórios, sem `..`, sem caracteres de controle, até `150` caracteres).
  - **Foto Principal e Coleta de Lixo Manual**:
    - A primeira foto anexada a um modelo torna-se automaticamente a foto principal (`is_primary = true`); ao remover a foto principal, a próxima foto ativa por `sort_order` é promovida automaticamente.
    - Remover um vínculo (`remove_model_media`) apenas desfaz o vínculo do modelo; o arquivo físico no `MediaStore` só é removido pela **coleta de lixo manual** (`collect_unreferenced_media`) quando nenhum vínculo ativo no tenant referencia o blob SHA-256.
  - **Importação em Lote de Fotos (`/catalog/import-photos`)**:
    - O nome do arquivo (sem extensão e normalizado) é casado automaticamente na ordem estrita: **1º SKU**, **2º Código do Modelo** e **3º EAN**, exibindo pré-visualização (`Casado`, `Ambíguo`, `Sem correspondência`) para revisão/correção manual antes da confirmação e processando cada arquivo individualmente sem abortar o lote.

---

### Backup e Restauração Offline Transacional (Fase 2N / ADR-0017)

- **RN-BKP-01 — Controle de Acesso e Permissão**: Exclusivo para usuários com permissão `backup:manage` (Proprietário por padrão).
- **RN-BKP-02 — Snapshot SQLite Consistente**: O banco SQLite é congelado atomicamente via `VACUUM INTO temp_path`, eliminando fragmentos de WAL sujos, seguido de `PRAGMA integrity_check` obrigatório antes de arquivar.
- **RN-BKP-03 — Pacote Híbrido Zip com Mídias Deduplicadas**: O backup empacota `manifest.json`, `database.sqlite` e todos os arquivos em `media/` preservando os hashes SHA-256 dos anexos de catálogo e branding.
- **RN-BKP-04 — Proteção Criptográfica Contra Adulteração**: Ao verificar ou restaurar, todos os hashes SHA-256 dos arquivos contidos no arquivo `.zip` são recomputados e validados contra o manifesto. Arquivos corrompidos ou adulterados são rejeitados.
- **RN-BKP-05 — Compatibilidade de Schema e Versão**: Backups cuja última migração aplicada seja maior que a versão máxima suportada pelo binário são marcados como incompatíveis e sua restauração é expressamente impedida.
- **RN-BKP-06 — Isolamento de Concorrência**: Operações de backup adquirem a trava RAII `BackupOperationGuard`. Operações concorrentes de backup/restauração e a coleta de lixo de mídias (`run_media_gc`) são bloqueadas enquanto a rotina estiver em execução.
- **RN-BKP-07 — Retenção Automatizada (Dias, Semanas e Meses)**: Ao concluir um backup com sucesso, a limpeza de retenção preserva no máximo um backup por dia nos últimos X dias, um por semana nas últimas Y semanas e um por mês nos últimos Z meses, eliminando cópias obsoletas de forma determinística.
- **RN-BKP-08 — Restauração a Frio na Inicialização**: A substituição do banco e da pasta de mídia é agendada em marcador atômico e executada exclusivamente a frio antes da inicialização de conexões e pools SQLite na inicialização do aplicativo, com criação prévia de salvaguarda de segurança (`.pre-restore.bak`).
