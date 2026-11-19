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

### 4. Validação Rigorosa no Core e Rejeição de EXIF

- **Imagens (`image/webp`, `image/jpeg`, `image/png`)**:
  - Validação de _magic bytes_ reais coerentes com o `mime_type` declarado.
  - Leitura segura de dimensões do cabeçalho via `imagesize` (ADR-0010): versão de exibição até **2048 px** no maior lado e **600 KB**; miniatura até **512 px** no maior lado e **60 KB**.
  - **Rejeição estrita de metadados EXIF**: o core inspeciona a estrutura de chunks/segmentos (chunk RIFF `EXIF` em WebP, marcador `APP1 Exif` `0xFFE1` em JPEG e chunk `eXIf` em PNG) e rejeita qualquer imagem que contenha EXIF.
  - No frontend, antes do envio, as imagens são redimensionadas em `<canvas>` (exibição até 1280 px, miniatura até 256 px) e reencodadas em WebP (com fallback JPEG), eliminando automaticamente metadados EXIF na origem.
- **Documentos (`application/pdf`)**:
  - Validação do cabeçalho `%PDF-` e limite máximo de **25 MB**.
- **Limites por Modelo**:
  - No máximo **12 fotos** e **20 documentos** ativos por modelo de equipamento.

### 5. Segurança de Exibição, PDFs e Proibição de Download por URL

1. **Proibição de Download Automático por URL**:
   - O aplicativo não baixa imagens nem documentos a partir de URLs externas na primeira versão, eliminando vetores de **SSRF**.
2. **Sem Protocolo `asset://` Amplo**:
   - O frontend obtém os bytes das imagens via comando autenticado e constrói `Blob URL`s gerenciadas por um cache **LRU em memória** (com limite de itens e de bytes totais e revogação via `URL.revokeObjectURL`).
3. **Documentos PDF Nunca Interpretados no App**:
   - PDFs nunca são renderizados ou abertos dentro da WebView do aplicativo.
   - Para abrir um documento, o backend grava uma cópia com nome saneado na pasta temporária exclusiva do app (`<temp>/eventrent-media-preview/`) e aciona o leitor padrão do sistema operacional (`tauri-plugin-opener`), ou permite exportação explícita via diálogo **"Salvar como..."** (`tauri-plugin-dialog`).
   - A pasta temporária de visualização é limpa automaticamente na inicialização e no encerramento do aplicativo.

### 6. Backup, Migração Local → Nuvem e Antivírus

- **Backup e Migração (Fase 2.6)**: o diretório `media/<tenant_id>/` faz parte integrante do pacote de backup local e do processo de migração do modo Local (SQLite) para Nuvem (PostgreSQL + Object Storage), com verificação de integridade pelo hash SHA-256 de cada `media_blob`.
- **Verificação de Vírus na Nuvem (Fase 2.5)**: no modo Nuvem multi-usuário, todo upload passará por fila/verificação de antivírus (ex.: ClamAV / scanner de bucket) antes de ficar disponível para download por outros usuários.
