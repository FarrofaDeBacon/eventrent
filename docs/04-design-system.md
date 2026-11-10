# 04 - Design System e Identidade Visual: EventRent

## 1. Princípios do Design System

- **Modernidade e Sofisticação**: Estética premium, clean, com foco em densidade de informação ideal para operadores de eventos e alta legibilidade.
- **Tokens Semânticos em OKLCH**: Todos os valores de cor utilizam o espaço de cor perceptualmente uniforme `oklch(L C H)` ou sua projeção mapeada em sRGB.
- **Agnóstico a Componentes**: Nenhum componente Svelte possui cores literais fixas (`bg-[#...]`, `text-[rgb(...)]`, etc.). As classes do Tailwind referenciam variáveis semânticas CSS (`var(--primary)`, `var(--background)`, `var(--radius)`, etc.).
- **Proibição Estrita de Cores Literais**: Verificada continuamente pelo script `scripts/check-no-hardcoded-colors.js` integrado ao `pnpm lint` e no pipeline de CI.
- **Modos Claro e Escuro**: Suportado nativamente via classes e variáveis CSS gerenciadas com `mode-watcher`.

## 2. Mapa de Variáveis Semânticas (CSS Tokens)

```css
:root {
  /* Escalas Neutras e Superfícies (Modo Claro) */
  --background: oklch(0.985 0.002 247.8);
  --foreground: oklch(0.145 0.015 260);
  --card: oklch(1 0 0);
  --card-foreground: oklch(0.145 0.015 260);
  --popover: oklch(1 0 0);
  --popover-foreground: oklch(0.145 0.015 260);
  --muted: oklch(0.96 0.005 260);
  --muted-foreground: oklch(0.55 0.015 260);
  --border: oklch(0.91 0.005 260);
  --input: oklch(0.91 0.005 260);

  /* Identidade do Tenant / Cores da Marca */
  --primary: oklch(0.55 0.22 260);
  --primary-foreground: oklch(0.99 0 0);
  --secondary: oklch(0.95 0.02 260);
  --secondary-foreground: oklch(0.2 0.03 260);
  --accent: oklch(0.92 0.03 260);
  --accent-foreground: oklch(0.2 0.03 260);

  /* Escala Semântica de 11 Passos (50 a 950) gerada dinamicamente */
  --primary-50: #...;
  --primary-100: #...;
  /* ... */
  --primary-950: #...;

  /* Feedback e Estados */
  --destructive: oklch(0.58 0.24 27.3);
  --destructive-foreground: oklch(0.99 0 0);
  --success: oklch(0.62 0.19 145);
  --success-foreground: oklch(0.99 0 0);
  --warning: oklch(0.75 0.18 75);
  --warning-foreground: oklch(0.15 0 0);

  /* Geometria */
  --radius: 0.5rem;
}

.dark {
  /* Escalas Neutras e Superfícies (Modo Escuro) */
  --background: oklch(0.13 0.015 260);
  --foreground: oklch(0.98 0.005 260);
  --card: oklch(0.17 0.015 260);
  --card-foreground: oklch(0.98 0.005 260);
  --popover: oklch(0.17 0.015 260);
  --popover-foreground: oklch(0.98 0.005 260);
  --muted: oklch(0.22 0.015 260);
  --muted-foreground: oklch(0.68 0.015 260);
  --border: oklch(0.26 0.015 260);
  --input: oklch(0.26 0.015 260);

  /* Identidade do Tenant em Modo Escuro */
  --primary: oklch(0.65 0.2 260);
  --primary-foreground: oklch(0.1 0.02 260);
  --secondary: oklch(0.24 0.02 260);
  --secondary-foreground: oklch(0.98 0.005 260);
  --accent: oklch(0.28 0.02 260);
  --accent-foreground: oklch(0.98 0.005 260);
}
```

## 3. Motor de Cores OKLCH e Contraste Acessível (`src/lib/theme/color.ts`)

- **Conversão de Espaços de Cor**:
  - `sRGB` normalizado $\leftrightarrow$ `sRGB Linear` (curva gamma padrão $2.4$).
  - `sRGB Linear` $\leftrightarrow$ `CIE XYZ / LMS` $\leftrightarrow$ `Oklab` $\leftrightarrow$ `Oklch`.
- **Mapeamento de Gamut**:
  - Implementado com busca binária de 20 iterações no componente Chroma ($C$).
  - Preserva fielmente a luminosidade perceptiva ($L$) e o matiz ($h$) sem gerar distorções ou cortes bruscos de canal.
- **Geração da Escala em 11 Passos**:
  - Passos: `50, 100, 200, 300, 400, 500, 600, 700, 800, 900, 950`.
  - Luminosidade estritamente decrescente de $L \approx 0.97$ (passo 50) até $L \approx 0.14$ (passo 950).
- **Cálculo de Contraste WCAG 2.1**:
  - Fórmula oficial de luminância relativa $Y = 0.2126 R_{lin} + 0.7152 G_{lin} + 0.0722 B_{lin}$.
  - Taxa de contraste $(Y_{claro} + 0.05) / (Y_{escuro} + 0.05)$.
  - Seleção automática de cor de texto (`primary-foreground`) garantindo nível AA ($\ge 4.5:1$).
  - Emissão de alerta em tempo real na tela de configurações caso a cor do tenant não atinja a proporção recomendada.

## 4. White Label e Regras de Logotipos

- **Formatos Permitidos**: PNG, JPEG e WebP codificados em Base64 / Data URI.
- **Assinatura Mágica de Arquivo (Magic Bytes)**:
  - Validação estrita no Core Rust (`BrandingValidator`) antes da gravação no banco.
  - PNG: `89 50 4E 47 0D 0A 1A 0A`
  - JPEG: `FF D8 FF`
  - WebP: `RIFF....WEBP`
- **Exclusão de SVG (Decisão de Segurança)**:
  - Arquivos SVG são rejeitados estritamente no frontend e no backend.
  - Motivação: sanitização complexa contra XSS / injeção XML e incompatibilidade com renderizadores de PDF em fases posteriores.
- **Limites de Tamanho e Redimensionamento**:
  - Logo Claro e Logo Escuro: redimensionados no frontend para no máximo 512px; limite decodificado de **256 KB**.
  - Ícone Quadrado: redimensionado para 128x128px; limite decodificado de **64 KB**.
- **Carregamento Instantâneo sem Flash (Zero FOUC)**:
  - Cache dos parâmetros de marca em `localStorage` (`eventrent:branding`).
  - Script síncrono no `<head>` do `app.html` que injeta as variáveis CSS antes da primeira pintura do DOM.

## 5. Auditoria de Marca

- As alterações de identidade visual ocorrem dentro da mesma transação atômica (`run_in_tx`) do banco SQLite.
- A trilha de auditoria (`audit_log`) registra apenas o sumário com metadados (ex.: alteração de nome, cores e indicadores booleanos de upload de logo), nunca armazenando bytes brutos ou Data URIs para evitar inchaço do banco e vazamento de payloads.
