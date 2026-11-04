<script lang="ts">
  import type { HTMLAttributes } from "svelte/elements";
  import type { Snippet } from "svelte";
  import { cn } from "../utils";

  interface Props extends HTMLAttributes<HTMLDivElement> {
    variant?: "default" | "secondary" | "destructive" | "success" | "outline";
    children?: Snippet;
  }

  let {
    variant = "default",
    class: className = "",
    children,
    ...restProps
  }: Props = $props();

  const variants = {
    default: "border-transparent bg-primary text-primary-foreground shadow-xs",
    secondary: "border-transparent bg-secondary text-secondary-foreground",
    destructive:
      "border-transparent bg-destructive text-destructive-foreground shadow-xs",
    success: "border-transparent bg-success text-success-foreground shadow-xs",
    outline: "border-border text-foreground",
  };
</script>

<div
  class={cn(
    "inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2",
    variants[variant],
    className,
  )}
  {...restProps}
>
  {#if children}
    {@render children()}
  {/if}
</div>
