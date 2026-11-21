# ADR-0014: Menor Privilégio em Capabilities do Tauri v2 (Remoção de Opener, FS, HTTP e Dialog do Frontend)

- **Status**: Aprovado
- **Data**: 2026-11-20
- **Contexto**: Fase 2M-rev (Segurança de Mídia e Menor Privilégio no Tauri)
- **Decisores**: Equipe de Arquitetura e Engenharia EventRent

---

## 1. Contexto e Problema

No ecossistema desktop Tauri v2, o sistema de _Capabilities_ define quais permissões e plugins do backend nativo podem ser invocados diretamente pelo contexto JavaScript da WebView (janela principal).

Em relatórios de auditoria de segurança externa e modelagem de ameaças contra clientes desktop:

1. Conceder permissões diretas como `opener:default`, `fs:*`, `http:*` ou `dialog:*` à janela web cria vetores graves de escalada de privilégios no caso de XSS ou injeção de scripts maliciosos.
2. A janela principal do EventRent **não realiza chamadas diretas** aos plugins `opener`, `fs`, `http` ou `dialog`.
3. Toda manipulação de documentos PDF (leitura segura, gravação em pasta temporária com extensão forçada `.pdf` e abertura no leitor nativo do sistema via `open_model_document_external`) e salvamento ("Salvar Como" com filtro e extensão `.pdf` garantida via `save_model_document_as`) é orquestrada e executada **estritamente em Rust** dentro de comandos autenticados e auditados.

## 2. Decisão

1. **Princípio do Menor Privilégio (PoLP) nas Capabilities**:
   - O arquivo [`src-tauri/capabilities/default.json`](../../src-tauri/capabilities/default.json) passa a conter **exclusivamente** `core:default` (gerenciamento básico do ciclo de vida da janela nativa).
   - Remoção completa de `opener:default`, `fs:*`, `http:*` e permissões diretas de `dialog:*` da janela principal.

2. **Justificativa de Cada Permissão**:
   - `core:default`: Permissão essencial do runtime do Tauri para fechamento, minimização e ciclo de vida da janela desktop.
   - `opener:*`: Proibido. A abertura de documentos no leitor padrão é executada no Rust via `app.opener().open_path(...)`, após validar que o caminho aponta para um arquivo temporário com extensão `.pdf` contido na pasta temporária isolada do app.
   - `fs:*`: Proibido. A persistência e leitura de mídias operam via `MediaStore` em Rust e canal binário customizado (`InvokeBody::Raw`).
   - `http:*`: Proibido. Toda comunicação do desktop na Fase 2M é local (SQLite).
   - `dialog:*`: Proibido. A seleção de local para gravação de PDF é acionada pelo comando Rust `save_model_document_as`, garantindo que o arquivo gravado receba extensão `.pdf` mesmo se o usuário digitar outra extensão.

3. **Garantia Contínua por Testes Automatizados**:
   - Foi implementado o teste unitário `guards::tests::test_tauri_capabilities_enforce_least_privilege` em `src-tauri/src/guards.rs`.
   - O teste analisa todos os manifestos de capabilities em tempo de compilação/teste e **falha imediatamente** caso qualquer permissão de `opener:`, `fs:`, `http:`, `dialog:` ou escopo de diretório amplo seja concedida à janela principal.

## 3. Consequências

- **Positivas**:
  - Elimina completamente o vetor de abuso de plugins Tauri a partir da WebView.
  - Alinhamento rigoroso com os padrões de segurança defensiva em profundidade (_defense-in-depth_).
  - Verificação automatizada contínua no CI (impossível reintroduzir permissões inadvertidamente).
- **Negativas / Mitigações**:
  - Toda nova funcionalidade que precise interagir com o SO deve expor um comando Rust com guardas estritas de sessão e auditoria, o que já é a convenção arquitetural mandatória do projeto.
