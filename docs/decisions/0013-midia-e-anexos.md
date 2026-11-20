# ADR-0013: Arquitetura de Mídia e Anexos (Fotos e Documentos de Equipamentos)

## Status

**Aceito** (Fase 2M)

## Contexto

O catálogo de equipamentos do **EventRent** precisa armazenar e exibir fotos (versão de exibição e miniatura) e documentos técnicos (manuais, datasheets, certificados em PDF) vinculados aos modelos de equipamento (`equipment_model`).

Armazenar binários grandes (`BLOB`) dentro do SQLite inflaria o arquivo principal do banco de dados, degradaria o desempenho de páginas/cache do motor relacional e dificultaria a transição para armazenamento de objetos em nuvem na Fase 2.5. Além disso, o processamento e a exibição de arquivos enviados pelo usuário trazem riscos de segurança importantes:

- **Path Traversal**: uso de nomes de arquivos enviados pelo cliente para compor caminhos no disco;
- **Vazamento de Metadados (EXIF/GPS)**: fotos tiradas em campo podem conter coordenadas GPS e números de série de câmeras/celulares;
- **Superfície de Ataque de PDF e Protocolo `asset://`**: renderizar PDFs dentro da WebView ou abrir escopo amplo no protocolo de arquivos locais do Tauri expõe o aplicativo a vulnerabilidades de parser e leitura arbitrária;
- **SSRF (Server-Side Request Forgery)**: baixar imagens automaticamente a partir de URLs informadas pelo usuário permitiria sondar redes internas.

---

## Decisão

Adotamos uma arquitetura de armazenamento de mídia **fora do banco relacional**, **endereçada por conteúdo (CAS — Content-Addressed Storage via SHA-256)** e isolada por `tenant_id`, desacoplada por uma trait `MediaStore` no `crates/core`.

### 1. Armazenamento Fora do Banco e Endereçamento por Conteúdo (SHA-256)

1. **Deduplicação por Tenant (`media_blob`)**:
   - Os bytes brutos nunca são gravados no SQLite.
   - Cada arquivo validado gera um hash **SHA-256** (64 caracteres hexadecimais minúsculos).
   - A tabela `media_blob` registra os metadados físicos únicos por `(tenant_id, sha256, mime_type)`, evitando duplicação em disco quando o mesmo manual PDF ou foto é anexado a múltiplos modelos do mesmo tenant.
2. **Layout no Sistema de Arquivos (`crates/media-fs`)**:
   - A chave de armazenamento e o caminho físico são derivados **exclusivamente** do `tenant_id` validado e do hash `sha256` (64 hex):
     `<dados_do_app>/media/<tenant_id>/<2_primeiros_hex_do_sha256>/<sha256>`
   - **Nunca** se utiliza o nome de arquivo enviado pelo usuário para compor caminhos no armazenamento permanente.
   - **Gravação Atômica e Idempotente**: escrita em arquivo temporário no mesmo diretório, `sync_all()` (`fsync`) e `rename` atômico para o destino final. Se o blob já existir com o mesmo hash, a operação é idempotente.

### 2. Abstração `MediaStore` no Core

- `crates/core` define a trait assíncrona `MediaStore` (`put_blob`, `get_blob`, `delete_blob`, `exists_blob`).
- Na Fase 2M (Desktop Local), a crate `crates/media-fs` implementa `MediaStore` sobre o sistema de arquivos local com verificação estrita de contenção de caminho (prevenção contra traversal).
- Na Fase 2.5 (Nuvem), uma implementação compatível com Object Storage (S3 / Cloudflare R2 / GCS) implementará a mesma trait `MediaStore` sem alterar regras de negócio nem entidades.

### 3. Tabelas de Vínculo Específicas por Entidade (Nunca Polimórficas)

- Não utilizamos tabelas polimórficas genéricas (`entity_type` + `entity_id` sem FK real).
- A estrutura separa:
  1. `media_blob`: registro físico deduplicado por `(tenant_id, sha256, mime_type)`.
  2. `media_asset`: ativo lógico com título, tipo (`image` ou `document`), `blob_id`, `thumb_blob_id` (para fotos), `doc_type` (`manual`, `datasheet`, `certificate`, `other`), `origin` (`manufacturer`, `own`, `website`, `other`), `credit_note`, `language`, `version_note` e `original_filename` saneado.
  3. `equipment_model_media`: tabela de vínculo específica de modelos de equipamento, com **chaves estrangeiras compostas** `(tenant_id, model_id) REFERENCES equipment_model(tenant_id, id)` e `(tenant_id, asset_id) REFERENCES media_asset(tenant_id, id)`, índice único parcial por vínculo ativo e índice único parcial garantindo **no máximo uma foto principal por modelo** (`WHERE is_primary = 1 AND deleted_at IS NULL`).
- A exclusão de um vínculo apenas remove/desativa a referência; o arquivo físico (`media_blob`) só é removido por **coleta de lixo explícita (`collect_unreferenced_media`)** quando nenhum ativo ou vínculo o referenciar.

### 4. Validação Rigorosa no Core e Rejeição Abrangente de Metadados (R6)

- **Imagens (`image/webp`, `image/jpeg`, `image/png`)**:
  - Validação de _magic bytes_ reais coerentes com o `mime_type` declarado.
  - Leitura segura de dimensões do cabeçalho via `imagesize` (ADR-0010): versão de exibição até **2048 px** no maior lado e **600 KB**; miniatura até **512 px** no maior lado e **60 KB**.
  - **Rejeição estrita e abrangente de metadados**:
    - **JPEG**: Rejeita qualquer segmento APP1 (0xE1: EXIF e XMP), APP13 (0xED: Photoshop / IPTC) e marcador COM (0xFE: comentários arbitrários).
    - **WebP**: Rejeita chunk RIFF `EXIF`, chunk RIFF `XMP ` e flags VP8X de EXIF (bit 3 / 0x08) ou XMP (bit 2 / 0x04).
    - **PNG**: Rejeita chunks ancilares `eXIf`, `tEXt`, `iTXt` e `zTXt`.
  - No frontend, antes do envio, as imagens são redimensionadas em `<canvas>` (exibição até 1280 px, miniatura até 256 px) e reencodadas em WebP (com fallback JPEG), eliminando metadados na origem.
- **Documentos (`application/pdf`)**:
  - Validação do cabeçalho `%PDF-` e limite máximo de **25 MB**.
  - **Extensão Fixa e Sanitização Rigorosa (R1)**: A extensão de arquivos abertos ou salvos externamente NUNCA vem do usuário; é sempre estritamente `.pdf`. Nomes reservados do Windows (CON, PRN, AUX, NUL, COM1-9, LPT1-9) e caracteres perigosos são neutralizados com fallback para `documento.pdf`.
  - **Caracteres Invisíveis e de Direção (R2)**: Rejeição de caracteres de formato e direção bidi (U+200B..U+200F, U+202A..U+202E, U+2066..U+2069, U+2028, U+2029, U+FEFF) em títulos, notas e nomes de arquivo.
- **Limites por Modelo**:
  - No máximo **12 fotos** e **20 documentos** ativos por modelo de equipamento.

### 5. Segurança de Exibição, PDFs e Menor Privilégio no Tauri

1. **Proibição de Download Automático por URL**:
   - O aplicativo não baixa imagens nem documentos a partir de URLs externas na primeira versão, eliminando vetores de **SSRF**.
2. **Sem Protocolo `asset://` Amplo e Menor Privilégio no Tauri (ADR-0014)**:
   - O frontend obtém os bytes das imagens via comando autenticado e constrói `Blob URL`s gerenciadas por um cache **LRU em memória** (com limite de itens e de bytes totais e revogação via `URL.revokeObjectURL`).
   - A janela principal opera com menor privilégio estrito (`capabilities/default.json` contendo apenas `["core:default"]`), sem permissões no frontend para `opener:*`, `fs:*`, `http:*` ou `dialog:*`.
3. **Documentos PDF e Abertura Segura no Sistema**:
   - PDFs nunca são renderizados ou abertos dentro da WebView do aplicativo.
   - Para abrir um documento, o backend grava uma cópia com stem saneado e extensão fixa `.pdf` na pasta temporária exclusiva do app (`<temp>/eventrent-media-preview/`) e aciona o leitor padrão do sistema operacional via Rust (`app.opener().open_path`), ou permite exportação explícita via diálogo nativo em Rust (`app.dialog().file().blocking_save_file()`).
   - A pasta temporária de visualização é limpa automaticamente na inicialização (subpastas > 1h) e no encerramento normal do aplicativo (R3).

### 6. Concorrência Segura e I/O Assíncrono no MediaStore (R7)

1. **I/O Não-Bloqueante (`spawn_blocking`)**:
   - Todas as operações de leitura, gravação e deleção no `FsMediaStore` executam em `tokio::task::spawn_blocking`, evitando congelar o executor com arquivos de até 25 MB.
2. **Limpeza de Temporários de Escrita**:
   - Na inicialização, arquivos temporários `*.tmp.*` com mais de 24 horas são removidos automaticamente.
3. **Coleta de Lixo Conciliada e Proteção de Concorrência**:
   - Na coleta do banco, blobs recém-criados (`created_at > now() - 10 minutos`) nunca são coletados, protegendo transações em andamento.
   - Na conciliação do disco, arquivos órfãos sem correspondência no banco só são apagados se tiverem mais de 1 hora de existência física.
   - No vínculo (`attach`), caso ocorra corrida com GC antigo, o backend re-verifica e regrava o arquivo a partir do buffer em memória.
4. **Criptografia Canônica (ADR-0015)**:
   - Adoção da crate padrão da indústria `sha2` com verificação dos 4 vetores canônicos do NIST.

### 6. Backup, Migração Local → Nuvem e Antivírus

- **Backup e Migração (Fase 2.6)**: o diretório `media/<tenant_id>/` faz parte integrante do pacote de backup local e do processo de migração do modo Local (SQLite) para Nuvem (PostgreSQL + Object Storage), com verificação de integridade pelo hash SHA-256 de cada `media_blob`.
- **Verificação de Vírus na Nuvem (Fase 2.5)**: no modo Nuvem multi-usuário, todo upload passará por fila/verificação de antivírus (ex.: ClamAV / scanner de bucket) antes de ficar disponível para download por outros usuários.
