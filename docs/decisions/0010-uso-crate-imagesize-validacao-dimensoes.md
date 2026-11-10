# ADR-0010: Adoção da Crate `imagesize` para Validação de Dimensões de Imagens de Marca

## Status

Aprovado

## Contexto

Na Fase 1B.1, o `BrandingValidator` passou a validar limites de tamanho em bytes (máximo 256 KB para logos e 64 KB para ícones) e magic bytes de arquivos PNG, JPEG e WebP.
No entanto, limitar apenas a contagem de bytes é insuficiente contra ataques de descompressão/dimensão forjada (ex.: "decompression bomb" ou imagens com cabeçalho declarando 30.000 × 30.000 pixels). Tais imagens ocupam poucos bytes em disco ou rede, mas estouram o consumo de memória na renderização do WebView/DOM e nos geradores de PDF da Fase 4.

Decodificar a imagem completa em pixels para descobrir suas dimensões é lento e contraproducente, consumindo a própria memória que pretendemos poupar.

## Decisão

Adotar a crate leve e sem dependências `imagesize = "0.13"` em `crates/core/Cargo.toml`.
A crate lê estritamente os primeiros bytes do cabeçalho da imagem (PNG IHDR, JPEG SOF0, WebP VP8X) para extrair largura e altura (`width` e `height`) sem instanciar buffers de pixels.

As regras de dimensão no `BrandingValidator` são:

1. **Logotipos (claro e escuro)**: largura máxima de 2048 pixels e altura máxima de 2048 pixels.
2. **Ícone quadrado da marca**: largura máxima de 512 pixels e altura máxima de 512 pixels.
3. **Dimensões nulas**: largura ou altura igual a zero são estritamente rejeitadas.
4. **Imagens truncadas ou corrompidas**: falha na leitura das dimensões pelo cabeçalho resulta em `CoreError::Validation`.

## Consequências

- Crate de cabeçalhos puramente informativa, ultra-leve e sem dependências pesadas de processamento gráfico.
- Proteção completa contra bombas de pixels (ex.: 30.000 × 30.000 px) e corrupção de cabeçalho.
- Preservação da performance do Core e segurança na exportação de documentos em PDF.
