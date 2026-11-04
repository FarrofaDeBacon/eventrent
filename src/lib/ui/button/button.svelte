<script lang="ts">
  import type { HTMLButtonAttributes, HTMLAnchorAttributes } from 'svelte/elements';
  import type { Snippet } from 'svelte';
  import { cn } from '../utils';

  type ButtonVariant = 'default' | 'secondary' | 'destructive' | 'outline' | 'ghost';
  type ButtonSize = 'sm' | 'md' | 'lg' | 'icon';

  interface BaseProps {
    variant?: ButtonVariant;
    size?: ButtonSize;
    href?: string;
    class?: string;
    children?: Snippet;
  }

  type Props = BaseProps & (HTMLButtonAttributes | HTMLAnchorAttributes);

  let {
    variant = 'default',
    size = 'md',
    href,
    class: className = '',
    children,
    ...restProps
  }: Props = $props();

  const baseStyles = 'inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-[var(--radius)] text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 select-none cursor-pointer';

  const variants = {
    default: 'bg-primary text-primary-foreground shadow-xs hover:opacity-90 active:scale-[0.99]',
    secondary: 'bg-secondary text-secondary-foreground shadow-xs hover:opacity-85 active:scale-[0.99]',
    destructive: 'bg-destructive text-destructive-foreground shadow-xs hover:opacity-90 active:scale-[0.99]',
    outline: 'border border-input bg-card text-foreground shadow-2xs hover:bg-muted hover:text-foreground active:scale-[0.99]',
    ghost: 'text-foreground hover:bg-muted active:scale-[0.99]'
  };

  const sizes = {
    sm: 'h-8 px-3 text-xs',
    md: 'h-9 px-4 py-2',
    lg: 'h-11 px-6 text-base',
    icon: 'h-9 w-9 p-0'
  };
</script>

{#if href}
  <a
    {href}
    class={cn(baseStyles, variants[variant], sizes[size], className)}
    {...(restProps as HTMLAnchorAttributes)}
  >
    {#if children}
      {@render children()}
    {/if}
  </a>
{:else}
  <button
    class={cn(baseStyles, variants[variant], sizes[size], className)}
    {...(restProps as HTMLButtonAttributes)}
  >
    {#if children}
      {@render children()}
    {/if}
  </button>
{/if}
