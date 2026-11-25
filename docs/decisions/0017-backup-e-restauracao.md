# ADR-0017: Arquitetura de Backup e Restauração Transacional Offline-First

## Status

Aceito (Fase 2N - Backup e Restauração)

## Contexto

O EventRent opera como uma aplicação desktop offline-first instalada localmente no ambiente do usuário. O estado completo da empresa — inventário de equipamentos, arquivos de fotos e especificações técnicas (PDFs), cadastros de clientes com documentos fiscais (CPF/CNPJ), contratos, locais e usuários — reside no disco local, distribuído entre:

1. O banco de dados relacional SQLite (`eventrent.db`) operando em modo WAL (`journal_mode = WAL`).
2. O armazenamento de objetos de mídia em sistema de arquivos (`media/`), endereçado por conteúdo (SHA-256) conforme ADR-0013.
3. O arquivo de configuração geral do aplicativo (`config.json`).

Em uma aplicação desktop sem sincronização obrigatória em nuvem, a perda de dados decorrente de falha de disco, infecção por malware, substituição acidental de arquivos ou erros catastróficos durante migrações de versão representa um risco de negócio existencial para a locadora de eventos.

Para garantir a continuidade operacional, integridade e resiliência dos dados, é necessária uma arquitetura robusta de **Backup e Restauração** que seja:

- **Consistente:** Capaz de capturar uma cópia instantânea sem lock prolongado e sem corrupção decorrente de arquivos abertos ou buffers em escrita.
- **Atômica:** Não deixando arquivos residuais ou incompletos caso haja falta de energia ou fechamento do processo.
- **Verificável:** Validando integridade física e lógica por meio de somas de verificação SHA-256 e testes de integridade do SQLite (`PRAGMA integrity_check`).
- **Segura e Segregada (LGPD):** Respeitando a Lei Geral de Proteção de Dados, restringindo o acesso exclusivamente ao papel de Proprietário (`backup:manage`) e protegendo contra travessia de diretórios (_path traversal_).

## Decisões

### 1. Escopo do Backup e Exclusões Estritas

O arquivo de backup gerado pelo EventRent encapsula a totalidade do estado persistente do inquilino local:

- **Incluídos no backup:**
  - Cópia consistente do banco de dados SQLite (`eventrent.db`).
  - Todo o diretório de mídias (`media/`), incluindo subdiretórios de fotos (originais, exibição, miniaturas) e documentos (manuais em PDF).
  - O arquivo de configuração do sistema (`config.json`).
  - O manifesto canônico de verificação (`manifest.json`).
- **Excluídos explicitamente:**
  - Arquivos de log de auditoria e aplicação (`logs/`, `*.log`).
  - Arquivos temporários e buffers em processamento (`*.tmp.*`, pastas temporárias do SO).
  - Arquivos de memória compartilhada e buffers ativos do SQLite (`eventrent.db-shm`, `eventrent.db-wal`).
  - Sockets de comunicação IPC e arquivos voláteis de runtime.

### 2. Cópia Consistente do Banco de Dados via `VACUUM INTO`

Em modo WAL, o SQLite mantém escritas recentes nos arquivos `-wal` e `-shm`. A cópia direta desses arquivos pelo sistema operacional enquanto conexões estiverem abertas gera risco iminente de cópia inconsistente ou corrompida.

- **Mecanismo Adotado:** O EventRent utiliza a instrução nativa do SQLite `VACUUM INTO '<temp_file>'`.
- **Garantias:**
  - Executa uma consolidação atômica e em ponto-no-tempo do banco de dados diretamente para um arquivo novo e autocontido.
  - Opera perfeitamente com conexões concorrentes ativas e em modo WAL, sem interrupção de transações de leitura.
  - O arquivo gerado pelo `VACUUM INTO` já vem desfragmentado e não possui dependências de arquivos `-wal` ou `-shm`.
- **Validação Imediata:** Imediatamente após a geração da cópia temporária e antes de compactá-la no arquivo final, o sistema conecta nessa base isolada e executa `PRAGMA integrity_check`, garantindo que nenhuma anomalia seja embutida no backup.

### 3. Sincronização e Bloqueio de Coleta de Lixo de Mídia

Os arquivos de mídia do EventRent são imutáveis e endereçados por hash SHA-256 (ADR-0013).

- **Sequência de Captura:** O banco de dados é congelado primeiro via `VACUUM INTO`, e os arquivos físicos do diretório `media/` são compactados imediatamente após. Como o banco de dados é a autoridade sobre quais mídias existem, qualquer blob inserido após o `VACUUM INTO` simplesmente não constará no banco e não quebrará referências.
- **Exclusão Mútua com Coleta de Lixo (GC):** A rotina de limpeza de mídias órfãs (`collect_unreferenced_media`) e qualquer processo de backup/restauração operam sob um mecanismo de bloqueio concorrente em memória (`RwLock`/Mutex no Core). A coleta de lixo é rejeitada com erro transitório caso um backup ou restauração esteja em andamento.

### 4. Formato do Pacote e Estratégia de Compressão

- **Extensão e Estrutura:** O pacote gerado é um arquivo único com extensão `.eventrent-backup`, baseado no formato padrão ZIP via crate Rust `zip`.
- **Justificativa da Dependência:** A crate `zip` (com `default-features = false` e `features = ["deflate"]`) é a biblioteca padrão e madura do ecossistema Rust para arquivos ZIP compactados. Ela elimina dependências de ferramentas externas do sistema operacional (como 7z ou zip nativo do Windows), garantindo determinismo entre sistemas operacionais.
- **Gravação Atômica:** O arquivo é criado inicialmente com extensão `.tmp.eventrent-backup` no diretório final de destino escolhido pelo usuário. Apenas após a gravação completa de todos os fluxos e do `manifest.json`, o arquivo temporário é renomeado atomicamente para sua extensão final `.eventrent-backup`. Caso o processo seja interrompido, nenhum arquivo corrompido ou parcial permanece com a extensão oficial.
- **Estratégia Híbrida de Compressão:**
  - `eventrent.db`, `config.json` e `manifest.json`: compactados com algoritmo `Deflate`, proporcionando alta taxa de compressão para tabelas de texto e SQL.
  - Fotos (WebP, JPEG, PNG) e Documentos (PDF): armazenados no arquivo ZIP no modo `Stored` (sem compressão adicional). Como esses formatos já possuem compressão nativa intrínseca, recompactá-los consumiria ciclos de CPU excessivos sem ganho mensurável de espaço em disco.

### 5. Estrutura do Manifesto (`manifest.json`)

O arquivo raiz do arquivo ZIP contém o manifesto canônico em formato JSON:

```json
{
  "manifest_version": 1,
  "app_version": "0.10.0",
  "created_at": "2026-11-23T01:00:00Z",
  "last_applied_migration": 10,
  "migrations_checksum": "a8f3b...64e",
  "database_integrity_checked": true,
  "files": [
    {
      "path": "database/eventrent.db",
      "byte_size": 2048000,
      "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    },
    {
      "path": "media/ab/c1/abc123...webp",
      "byte_size": 154200,
      "sha256": "4b227777d4dd1fc61c6f884f48641d02b4d121d3fd328cb08b5531fcacdabf8a"
    }
  ],
  "table_counts": {
    "app_user": 3,
    "customer": 12,
    "equipment_model": 45,
    "venue": 8,
    "audit_log": 120
  }
}
```

### 6. Restauração Segura em Inicialização Fria (Cold Restore)

Restaurar um banco de dados e arquivos de mídia enquanto o aplicativo está em execução representa risco grave de corrupção devido a conexões SQLx ativas, arquivos de cache e pools de threads.

- **Mecanismo de Inicialização Fria:**
  1. O usuário aciona "Restaurar Backup" na interface (restrito a `backup:manage`).
  2. O backend Rust valida o arquivo selecionado, inspeciona o `manifest.json` e verifica se os hashes SHA-256 e o esquema são compatíveis.
  3. O backend grava um marcador seguro `restore_pending.json` no diretório de dados do aplicativo e solicita o reinício controlado do aplicativo.
  4. Na próxima inicialização (antes de inicializar os pools do SQLite e antes de abrir o banco de dados para a interface):
     - O EventRent detecta o marcador `restore_pending.json`.
     - **Compatibilidade de Versão:** Se o `last_applied_migration` do backup for **maior** que a maior migração suportada pelo binário atual, a restauração é **rejeitada imediatamente**, impedindo corrupção por versão de schema incompatível. Se o schema for **anterior**, o sistema aceita o backup e executará as migrações automáticas subsequentes.
     - **Salvaguarda Automática (Pre-Restore Snapshot):** Antes de qualquer exclusão ou substituição de arquivos, o sistema cria automaticamente um backup completo do estado atual com o nome `pre-restore_<timestamp>.eventrent-backup`.
     - **Substituição Atômica:** O banco anterior e a pasta `media/` são substituídos pelos arquivos validados do backup. Em caso de qualquer erro de I/O ou hash inválido, o estado anterior é restaurado a partir do snapshot de salvaguarda.
     - O marcador `restore_pending.json` é removido e o boot prossegue normalmente.

### 7. Backup Pré-Atualização Automático

Antes de qualquer execução de novas migrações de banco (`migrate!`) durante a inicialização de uma versão atualizada do aplicativo, o EventRent executa automaticamente um backup pré-atualização silencioso. Caso a migração falhe, o usuário tem a garantia de que seus dados anteriores permanecem intactos e restauráveis.

### 8. Agendamento e Política de Retenção

- **Agendamento Inteligente:** Ao abrir o aplicativo, o sistema verifica a data do último backup bem-sucedido. Se tiver transcorrido mais de 24 horas (ou se nunca tiver sido feito), um backup agendado é iniciado em segundo plano (`spawn_blocking`).
- **Política de Retenção Canônica:**
  - Padrão do sistema: retenção de **7 backups diários**, **4 semanais** e **6 mensais**.
  - **Função Pura e Testável:** O cálculo de quais arquivos devem ser retidos ou expurgados é implementado como uma função pura no `eventrent-core` recebendo um relógio injetável (`Clock` trait).
  - **Proteção contra Deleção Indevida:** A rotina de limpeza de backups antigos só apaga arquivos que:
    1. Residam no diretório configurado de backup.
    2. Possuam a extensão `.eventrent-backup`.
    3. Contenham um `manifest.json` válido gerado pelo EventRent. Arquivos pessoais do usuário presentes no mesmo diretório são estritamente ignorados.

### 9. Segurança, LGPD e Permissões

- **Permissão Nova:** Criada a permissão canônica `backup:manage`. Ela é atribuída **exclusivamente ao perfil `Proprietário` (`owner`)** no catálogo `system-roles.json` e na matriz de permissões. Nenhum outro perfil (Administrador, Gerente, Técnico, etc.) tem permissão para gerar, baixar ou restaurar backups.
- **Validação de Caminhos (Anti-Path Traversal):**
  - O diretório de destino de backup não pode ser o diretório de dados internos do aplicativo (evitando recursão infinita e corrupção).
  - O seletor de pastas e arquivos é acionado através de chamadas seguras do Rust via `tauri-plugin-dialog`, sem expor acesso direto ao sistema de arquivos no Webview.
- **Aviso de Privacidade (LGPD):** A tela de configurações exibe aviso explícito de que o arquivo de backup contém dados pessoais e bancários de clientes e colaboradores, sendo de responsabilidade do Proprietário o armazenamento seguro do arquivo gerado em mídias confiáveis.
- **Criptografia Simétrica (Backlog Fase 3):** A criptografia com senha (AES-256-GCM / Argon2id) fica registrada como funcionalidade de aprofundamento para a Fase 3, mantendo o formato ZIP aberto e inspecionável nesta fase inicial.

### 10. Persistência de Configuração sem Novas Migrações

Em conformidade com a diretriz de congelamento de migrações existentes da Fase 2N, a configuração de backup (pasta de destino, agendamento ativo e parâmetros de retenção) é armazenada na tabela já existente `tenant_setting` através de chave de configuração dedicada (`backup_settings`), eliminando a necessidade de migrações adicionais no SQLite.

## Consequências

### Positivas

- Resiliência completa e garantia de recuperação de desastres para a operação da locadora.
- Impossibilidade de corrupção do banco durante o backup graças ao `VACUUM INTO`.
- Restauração segura e limpa via inicialização fria com snapshot de salvaguarda prévia automática.
- Conformidade estrita com o princípio do menor privilégio (somente Proprietário).
- Zero alterações ou quebras nas 10 migrações existentes e congeladas.

### Negativas / Mitigações

- O processo de restauração requer o reinício do aplicativo, o que é mitigado por mensagens claras ao usuário e confirmação prévia em modal bloqueante.
- O arquivo `.eventrent-backup` sem senha contém dados abertos no formato ZIP; mitigado pelo aviso ostensivo de conformidade LGPD e restrição de acesso apenas ao Proprietário.
