<script lang="ts">
  import {
    Zap,
    ShieldCheck,
    Palette,
    Database,
    ArrowRight,
    UserCog,
  } from "lucide-svelte";
  import { appSession, hasPermission, PERMISSIONS } from "$lib/data";
  import { Card, CardHeader, CardTitle, CardContent } from "$lib/ui/card";
  import { Button } from "$lib/ui/button";
  import { Badge } from "$lib/ui/badge";
  import { _ } from "$lib/i18n";
</script>

<div class="space-y-8">
  <div
    class="relative overflow-hidden rounded-[var(--radius)] border border-border bg-gradient-to-br from-card via-card to-primary/5 p-8 shadow-xs"
  >
    <div class="flex flex-col gap-3 max-w-2xl">
      <div class="flex items-center gap-2">
        <Badge variant="default">{$_("dashboard.phaseBadge")}</Badge>
        <span class="text-xs text-muted-foreground"
          >{$_("dashboard.techStack")}</span
        >
      </div>
      <h2 class="text-3xl font-bold tracking-tight text-foreground">
        {$_("dashboard.title")}
      </h2>
      <p class="text-muted-foreground text-sm leading-relaxed">
        {$_("dashboard.description")}
      </p>
      <div class="pt-2 flex flex-wrap gap-3">
        <Button href="/configuracoes/aparencia">
          {$_("dashboard.ctaAppearance")}
          <ArrowRight class="h-4 w-4" />
        </Button>

        {#if hasPermission(appSession.session, PERMISSIONS.USERS_VIEW)}
          <Button variant="outline" href="/configuracoes/usuarios">
            <UserCog class="h-4 w-4" />
            {$_("dashboard.ctaUsers")}
          </Button>
        {/if}
      </div>
    </div>
  </div>

  <div class="grid gap-6 sm:grid-cols-2 lg:grid-cols-4">
    <Card>
      <CardHeader>
        <div class="flex items-center justify-between">
          <CardTitle class="text-sm font-medium"
            >{$_("dashboard.cardPerfTitle")}</CardTitle
          >
          <Zap class="h-4 w-4 text-primary" />
        </div>
      </CardHeader>
      <CardContent>
        <div class="text-2xl font-bold text-foreground">
          {$_("dashboard.cardPerfValue")}
        </div>
        <p class="text-xs text-muted-foreground mt-1">
          {$_("dashboard.cardPerfDesc")}
        </p>
      </CardContent>
    </Card>

    <Card>
      <CardHeader>
        <div class="flex items-center justify-between">
          <CardTitle class="text-sm font-medium"
            >{$_("dashboard.cardDbTitle")}</CardTitle
          >
          <Database class="h-4 w-4 text-success" />
        </div>
      </CardHeader>
      <CardContent>
        <div class="text-2xl font-bold text-foreground">
          {$_("dashboard.cardDbValue")}
        </div>
        <p class="text-xs text-muted-foreground mt-1">
          {$_("dashboard.cardDbDesc")}
        </p>
      </CardContent>
    </Card>

    <Card>
      <CardHeader>
        <div class="flex items-center justify-between">
          <CardTitle class="text-sm font-medium"
            >{$_("dashboard.cardTokensTitle")}</CardTitle
          >
          <Palette class="h-4 w-4 text-primary" />
        </div>
      </CardHeader>
      <CardContent>
        <div class="text-2xl font-bold text-foreground">
          {$_("dashboard.cardTokensValue")}
        </div>
        <p class="text-xs text-muted-foreground mt-1">
          {$_("dashboard.cardTokensDesc")}
        </p>
      </CardContent>
    </Card>

    <Card>
      <CardHeader>
        <div class="flex items-center justify-between">
          <CardTitle class="text-sm font-medium"
            >{$_("dashboard.cardPortsTitle")}</CardTitle
          >
          <ShieldCheck class="h-4 w-4 text-accent" />
        </div>
      </CardHeader>
      <CardContent>
        <div class="text-2xl font-bold text-foreground">
          {$_("dashboard.cardPortsValue")}
        </div>
        <p class="text-xs text-muted-foreground mt-1">
          {$_("dashboard.cardPortsDesc")}
        </p>
      </CardContent>
    </Card>
  </div>
</div>
