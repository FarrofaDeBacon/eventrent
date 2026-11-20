# ADR-0015: Adoção da Crate `sha2` e Verificação com Vetores NIST

- **Status**: Aprovado
- **Data**: 2026-11-20
- **Contexto**: Fase 2M-rev (Criptografia Padronizada e Desempenho de Hash)
- **Decisores**: Equipe de Arquitetura e Engenharia EventRent

---

## 1. Contexto e Problema

Na versão inicial da Fase 2M, uma rotina pura de SHA-256 (FIPS 180-4) havia sido implementada manualmente em `crates/core/src/domain/media.rs` com o intuito de evitar introdução de dependências externas.
No entanto, a revisão de arquitetura apontou:

1. Códigos criptográficos escritos manualmente acarretam risco desnecessário de manutenção, suscetibilidade a sutilezas de implementação e menor eficiência de otimização por hardware (SIMD / extensões SHA-NI).
2. A crate `sha2` (do projeto RustCrypto) já constava no grafo de dependências transitivas do workspace no `Cargo.lock` (versão 0.10.8).
3. Para arquivos de mídia grandes (como manuais PDF de até 25 MB), a computação síncrona dentro de tarefas assíncronas pode bloquear threads de execução do Tokio se não for offloadada via `tokio::task::spawn_blocking`.

## 2. Decisão

1. **Substituição da Implementação Caseira pela Crate `sha2`**:
   - Adicionada a dependência direta `sha2 = "0.10"` em `crates/core/Cargo.toml`, reutilizando a versão auditada e já consolidada no `Cargo.lock`.
   - Remoção de toda a implementação manual de ~100 linhas em `crates/core/src/domain/media.rs`.
   - Utilização de `sha2::{Digest, Sha256}` em `compute_sha256_hex`, garantindo consumo direto de fatias de memória sem alocações desnecessárias.

2. **Offloading Assíncrono (`tokio::task::spawn_blocking`)**:
   - Disponibilizada a função `compute_sha256_hex_async(data)` que executa o cálculo de hashes para buffers expressivos em uma thread dedicada do threadpool de I/O bloqueante do Tokio, prevenindo congelamento da thread de execução do runtime.

3. **Verificação de Conformidade com Vetores Oficiais do NIST**:
   - Foram adicionados testes unitários automatizados cobrindo os 4 vetores de teste canônicos do NIST (FIPS 180-4):
     - Mensagem vazia `""` -> `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`
     - `"abc"` -> `ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad`
     - `"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"` -> `248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1`
     - 1.000.000 repetições de `"a"` -> `cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0`

## 3. Consequências

- **Positivas**:
  - Código criptográfico auditado pela comunidade, testado mundialmente e com aproveitamento de aceleração por hardware onde disponível.
  - Não introduz dependências inéditas no ecossistema (já presente no `Cargo.lock`).
  - 100% de conformidade validada com a suíte de testes do NIST.
