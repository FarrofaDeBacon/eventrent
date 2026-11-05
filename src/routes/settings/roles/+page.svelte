<script lang="ts">
  import { onMount } from "svelte";
  import { Shield, CheckCircle2, Lock } from "lucide-svelte";
  import { getUserRepository, type RoleWithPermissions } from "$lib/data";
  import { _ } from "$lib/i18n";
  import { Badge } from "$lib/ui/badge";
  import {
    Card,
    CardContent,
    CardDescription,
    CardHeader,
    CardTitle,
  } from "$lib/ui/card";
  import { EmptyState, ErrorState, LoadingState } from "$lib/ui";

  let isLoading = $state(true);
  let loadError = $state<string | null>(null);
  let roles = $state<RoleWithPermissions[]>([]);

  async function loadRoles() {
    isLoading = true;
    loadError = null;
    try {
      const repo = getUserRepository();
      roles = await repo.listRoles();
    } catch (err) {
      loadError = err instanceof Error ? err.message : String(err);
    } finally {
      isLoading = false;
    }
  }

  onMount(() => {
    loadRoles();
  });
</script>

<div class="space-y-6">
  <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
    <div>
      <h2 class="text-2xl font-bold tracking-tight text-foreground">
        {$_("roles.title")}
      </h2>
      <p class="text-sm text-muted-foreground mt-1">
        {$_("roles.description")}
      </p>
    </div>

    <div
      class="inline-flex items-center gap-2 rounded-[var(--radius)] border border-border bg-muted/40 px-3 py-1.5 text-xs font-medium text-muted-foreground"
    >
      <Lock class="h-3.5 w-3.5" />
      <span>{$_("roles.readonlyNotice")}</span>
    </div>
  </div>

  {#if isLoading}
    <LoadingState message={$_("roles.loading")} />
  {:else if loadError}
    <ErrorState message={loadError} onRetry={loadRoles} />
  {:else if roles.length === 0}
    <EmptyState />
  {:else}
    <div class="grid gap-6 md:grid-cols-2">
      {#each roles as role (role.id)}
        <Card data-testid="role-card-{role.slug}">
          <CardHeader>
            <div class="flex items-center justify-between gap-2">
              <CardTitle class="flex items-center gap-2 text-base">
                <Shield class="h-4 w-4 text-primary" />
                <span>{role.name}</span>
              </CardTitle>
              <div class="flex items-center gap-2">
                {#if role.is_system}
                  <Badge variant="secondary">{$_("roles.systemBadge")}</Badge>
                {/if}
                <Badge variant="outline">
                  {role.permissions.length}
                  {$_("roles.permissionsCount")}
                </Badge>
              </div>
            </div>
            {#if role.description}
              <CardDescription>{role.description}</CardDescription>
            {/if}
          </CardHeader>
          <CardContent>
            <div class="flex flex-wrap gap-2">
              {#each role.permissions as perm (perm)}
                <div
                  class="inline-flex items-center gap-1.5 rounded-[var(--radius)] border border-border bg-muted/30 px-2.5 py-1 text-xs text-foreground"
                >
                  <CheckCircle2 class="h-3.5 w-3.5 text-success shrink-0" />
                  <span>{$_(`permissions.${perm}`, { default: perm })}</span>
                  <code class="text-[10px] font-mono text-muted-foreground">
                    ({perm})
                  </code>
                </div>
              {/each}
            </div>
          </CardContent>
        </Card>
      {/each}
    </div>
  {/if}
</div>
