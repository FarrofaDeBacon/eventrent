# ADR-0005: Logs em Arquivo com Rotação Diária, Diálogo Nativo de Erro e Instância Única

## Contexto

Durante a revisão da Fase 0 (Fase 0.2), foram identificados três requisitos operacionais essenciais para o ciclo de vida e suporte da aplicação desktop em produção:

1. **Diagnóstico e Suporte a Clientes**: Em compilação `release` no Windows, aplicações Tauri não abrem console (`stdout`). Qualquer falha de inicialização ou erro de banco precisa ser registrado em arquivo persistente na pasta de dados do aplicativo com rotação diária.
2. **Feedback Visual em Falha Crítica de Boot**: Caso o diretório de dados ou o banco SQLite falhe ao iniciar (antes mesmo de o WebView carregar), o usuário precisa receber uma caixa de diálogo nativa do sistema operacional informando o motivo do erro e o caminho do arquivo de log, encerrando o processo de forma controlada.
3. **Proteção contra Concorrência de Processos**: Impedir que o usuário abra duas instâncias simultâneas do aplicativo desktop apontando para o mesmo arquivo SQLite local, focando a janela existente quando uma segunda instância for acionada.

## Decisão

Adotar as seguintes crates oficiais do ecossistema Tokio/Tracing e Tauri 2 em `src-tauri`:

- `tracing-appender` (`0.2`): Escrita de logs em arquivo na pasta de dados do aplicativo (`<app_data_dir>/logs/`) com rotação diária (`rolling::daily`), combinada com saída em `stdout` durante o desenvolvimento.
- `tauri-plugin-dialog` (`2`): Exibição de caixa de diálogo nativa do sistema operacional em caso de erro fatal na inicialização da aplicação.
- `tauri-plugin-single-instance` (`2`): Garantia de instância única do processo desktop, restaurando e trazendo para foco a janela principal existente quando uma nova abertura é solicitada.

## Consequências

- **Positivas**: Suporte técnico rastreável no cliente final via arquivos de log diários, ausência de falhas silenciosas no boot e prevenção de bloqueios ou corrupção por múltiplos processos acessando o mesmo banco SQLite.
- **Negativas/Atenção**: O `WorkerGuard` do `tracing-appender` deve ser mantido vivo durante toda a execução da aplicação para garantir o flush completo dos logs antes do encerramento.
