# ADR-0009: Adoção da Crate `base64` para Validação Estrita de Imagens de Marca

## Status

Aprovado

## Contexto

Na Fase 1B.1, implementamos a validação de imagens de marca (logo claro, logo escuro e ícone quadrado) recebidas via Data URI (`data:image/<formato>;base64,<dados>`).
O decodificador manual de Base64 implementado anteriormente foi apontado como potencial ponto de vulnerabilidade (edge cases de padding, limites de memória e caracteres espúrios).

A crate oficial `base64` (versão 0.22.1) já se encontra presente e congelada no `Cargo.lock` do projeto como dependência indireta de outros componentes.

## Decisão

Declarar explicitamente a dependência `base64 = "0.22"` em `crates/core/Cargo.toml` utilizando a engine estrita `base64::engine::general_purpose::STANDARD`.
Estabelecer as seguintes regras de validação no `BrandingValidator`:

1. **Rejeição Pré-Decodificação pelo Tamanho Codificado**: O tamanho da string base64 é conferido antes de qualquer tentativa de decodificação (`MAX_ENCODED_LEN = ((max_bytes + 2) / 3) * 4`), prevenindo alocação excessiva de memória (ataques de negação de serviço / zip bomb).
2. **Exigência de Prefixo Estrito**: O Data URI deve iniciar obrigatoriamente e exatamente com `data:image/png;base64,`, `data:image/jpeg;base64,` ou `data:image/webp;base64,`, sem parâmetros de charset ou espaços adicionais.
3. **Decodificação Estrita**: Utilização de `BASE64_STANDARD.decode()`, que rejeita automaticamente caracteres fora do alfabeto RFC 4648, espaços, quebras de linha (`\r`, `\n`) ou padding incorreto.
4. **Verificação de Magic Bytes contra o MIME Declarado**: Os primeiros bytes do payload decodificado são comparados estritamente contra as assinaturas PNG (`\x89PNG\r\n\x1a\n`), JPEG (`\xFF\xD8\xFF`) e WebP (`RIFF....WEBP`).
5. **Limites Máximos Decodificados**: 256 KB para logos e 64 KB para ícone.

## Consequências

- Zero novas dependências introduzidas no `Cargo.lock`.
- Eliminação completa de lógica de decodificação manual.
- Garantia de segurança contra enumeração, injeção de scripts (SVG disfarçado de imagem) e estouro de memória.
