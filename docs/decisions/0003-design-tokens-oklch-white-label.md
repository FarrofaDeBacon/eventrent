# ADR-0003: Design Tokens em Espaço OKLCH e Estratégia White Label

## Contexto

O EventRent é um SaaS comercializado no modelo B2B. As empresas contratantes exigem personalização com suas próprias marcas (logotipo, cores primárias e secundárias). Ao mesmo tempo, o sistema deve manter excelente contraste e acessibilidade em modos claro e escuro sem duplicação de componentes.

## Decisão

1. Adotar o espaço de cor perceptualmente uniforme **OKLCH** para todos os tokens de cor da aplicação.
2. Não permitir cores arbitrárias fixas nos componentes de UI. Todos os estilos consomem variáveis semânticas CSS (`var(--primary)`, `var(--background)`, `var(--muted)`, etc.).
3. Desenvolver o utilitário `applyBranding` que injeta dinamicamente no elemento `:root` as variáveis da marca do tenant ativo.

## Consequências

- Positivas: Consistência visual absoluta, transição de temas sem reflows ou re-renders custosos, suporte a white label sem tocar no código dos componentes.
- Negativas/Atenção: Requer suporte a navegadores/webviews modernos (garantido pelo WebView2 no Windows e WebKit modernos no macOS/Linux).
