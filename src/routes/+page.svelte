<script lang="ts">
  import { Zap, ShieldCheck, Palette, Database, ArrowRight, Sparkles } from 'lucide-svelte';
  import { Card, CardHeader, CardTitle, CardDescription, CardContent, CardFooter } from '$lib/ui/card';
  import { Button } from '$lib/ui/button';
  import { Badge } from '$lib/ui/badge';
  import { _ } from '$lib/i18n';
</script>

<div class="space-y-8 animate-in fade-in-50">
  <!-- Banner de Boas-vindas -->
  <div class="relative overflow-hidden rounded-[var(--radius)] border border-border bg-gradient-to-br from-card via-card to-primary/5 p-8 shadow-xs">
    <div class="flex flex-col gap-3 max-w-2xl">
      <div class="flex items-center gap-2">
        <Badge variant="default">Fase 0 Concluída</Badge>
        <span class="text-xs text-muted-foreground">Tauri 2 + Svelte 5 + Rust Core</span>
      </div>
      <h2 class="text-3xl font-bold tracking-tight text-foreground">
        EventRent — Gestão de Locação para Eventos
      </h2>
      <p class="text-muted-foreground text-sm leading-relaxed">
        Fundação arquitetural desktop construída para velocidade extrema (&lt; 1s de inicialização, &lt; 100ms em telas), persistência SQLite com migrações integradas e suporte a White Label dinâmico em OKLCH.
      </p>
      <div class="pt-2 flex flex-wrap gap-3">
        <Button href="/configuracoes/aparencia">
          Configurar Identidade Visual
          <ArrowRight class="h-4 w-4" />
        </Button>
      </div>
    </div>
  </div>

  <!-- Cards de Arquitetura e Pilares -->
  <div class="grid gap-6 sm:grid-cols-2 lg:grid-cols-4">
    <Card>
      <CardHeader>
        <div class="flex items-center justify-between">
          <CardTitle class="text-sm font-medium">Performance Extrema</CardTitle>
          <Zap class="h-4 w-4 text-primary" />
        </div>
      </CardHeader>
      <CardContent>
        <div class="text-2xl font-bold text-foreground">&lt; 100 ms</div>
        <p class="text-xs text-muted-foreground mt-1">Interações e renderizações reativas</p>
      </CardContent>
    </Card>

    <Card>
      <CardHeader>
        <div class="flex items-center justify-between">
          <CardTitle class="text-sm font-medium">Persistência SQLite</CardTitle>
          <Database class="h-4 w-4 text-success" />
        </div>
      </CardHeader>
      <CardContent>
        <div class="text-2xl font-bold text-foreground">SQLx Core</div>
        <p class="text-xs text-muted-foreground mt-1">Migrações automáticas embutidas</p>
      </CardContent>
    </Card>

    <Card>
      <CardHeader>
        <div class="flex items-center justify-between">
          <CardTitle class="text-sm font-medium">Tokens OKLCH</CardTitle>
          <Palette class="h-4 w-4 text-primary" />
        </div>
      </CardHeader>
      <CardContent>
        <div class="text-2xl font-bold text-foreground">White Label</div>
        <p class="text-xs text-muted-foreground mt-1">Logo e cores dinâmicas no :root</p>
      </CardContent>
    </Card>

    <Card>
      <CardHeader>
        <div class="flex items-center justify-between">
          <CardTitle class="text-sm font-medium">Fronteira de Dados</CardTitle>
          <ShieldCheck class="h-4 w-4 text-accent" />
        </div>
      </CardHeader>
      <CardContent>
        <div class="text-2xl font-bold text-foreground">Ports & Adapters</div>
        <p class="text-xs text-muted-foreground mt-1">Telas desacopladas do transporte</p>
      </CardContent>
    </Card>
  </div>
</div>
