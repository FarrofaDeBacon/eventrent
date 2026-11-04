# ADR-0002: Persistência Local com SQLite via SQLx e Crate Core Isolada

## Contexto

O sistema precisa de armazenamento relacional ACID local com suporte a transações complexas para disponibilidade de equipamentos. Além disso, a aplicação precisa evoluir para PostgreSQL no backend em nuvem sem reescrever regras de negócio.

## Decisão

1. Isolar todo o domínio, entidades e persistência em um crate Rust separado: `crates/core`, sem nenhuma dependência do Tauri.
2. Utilizar `sqlx` assíncrono com driver SQLite e modo WAL (`Write-Ahead Logging`).
3. Versionar migrações em `crates/core/migrations`, executadas automaticamente na inicialização via `sqlx::migrate!`.
4. Escrever SQL estritamente compatível e portável com PostgreSQL (evitando dialetos proprietários).
5. Padronizar identificadores como UUID v7 armazenados em texto (`TEXT`), permitindo ordenação natural cronológica e geração distribuída de IDs.

## Consequências

- Positivas: Testes de unidade rápidos com SQLite em memória (`sqlite::memory:`), reutilização de 100% do código de domínio em um futuro servidor web, zero acoplamento do banco com o frontend ou o framework de janela Tauri.
- Negativas/Atenção: Necessidade de validação constante de portabilidade das queries SQL para evitar recursos exclusivos de SQLite.
