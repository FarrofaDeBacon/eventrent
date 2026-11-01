# ADR-0001: Arquitetura Desktop First com Tauri 2 e SvelteKit

## Contexto
O EventRent é um sistema SaaS de locação de equipamentos voltado a empresas de eventos, exigindo altíssima performance (abertura < 1s, resposta < 100ms) e confiabilidade operacional mesmo em galpões com oscilação ou ausência de conectividade. Futuramente, uma versão online em nuvem será disponibilizada.

## Decisão
1. Adotar **Tauri 2** com backend em Rust para a versão desktop, garantindo consumo mínimo de memória RAM (< 80MB) e velocidade de processamento nativa.
2. Adotar **SvelteKit 2** configurado com `@sveltejs/adapter-static` como frontend SPA estático.
3. Estabelecer a fronteira de dados em `src/lib/data`: as telas consomem interfaces tipadas que hoje chamam comandos Tauri via `invoke`. No futuro, a versão Web utilizará a mesma interface comunicando-se via HTTP sem refatorar a interface.

## Consequências
- Positivas: Binário leve, inicialização ultrarrápida, segurança de memória do Rust, facilidade de migração para Web.
- Negativas/Atenção: Necessidade de manter compilação e CI para plataformas desktop (Windows, Linux, macOS).
