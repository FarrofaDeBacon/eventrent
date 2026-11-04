# 04 - Design System e Identidade Visual: EventRent

## 1. Princípios do Design System

- **Modernidade e Sofisticação**: Estética premium, clean, com foco em densidade de informação ideal para operadores de eventos e alta legibilidade.
- **Tokens Semânticos em OKLCH**: Todos os valores de cor utilizam o espaço de cor perceptualmente uniforme `oklch(...)`.
- **Agnóstico a Componentes**: Nenhum componente Svelte possui cores fixas ou raios de borda em código. As classes do Tailwind referenciam variáveis semânticas CSS (`var(--primary)`, `var(--background)`, `var(--radius)`, etc.).
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

## 3. Aplicação Dinâmica de Branding

A função utilitária `applyBranding(branding)` em `src/lib/theme/branding.ts` injeta dinamicamente as definições da marca no elemento raiz (`document.documentElement`), atualizando cores primárias e logo em tempo real sem necessidade de reload.

## 4. Componentes Base da Fase 0

- **Button**: Variantes primária, secundária, contorno (outline), fantasma (ghost), destrutiva, tamanhos sm, md, lg.
- **Input**: Campo de texto com suporte a ícones, rótulo e estados de validação/erro.
- **Card**: Container de superfície para blocos de informação com cabeçalho, conteúdo e rodapé.
- **Badge**: Indicador de status (default, secondary, destructive, success, outline).
- **Dialog / Modal**: Janela modal acessível baseada no `bits-ui`.
- **Toast**: Notificações flutuantes temporárias integradas com `svelte-sonner`.
- **Estados Padrão**:
  - `EmptyState`: Mensagem amigável com ilustração/ícone e ação quando não há itens.
  - `LoadingState`: Indicador de carregamento suave com skeleton/spinner.
  - `ErrorState`: Apresentação clara de falhas operacionais com botão de repetição.
