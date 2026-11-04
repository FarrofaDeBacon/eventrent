<script lang="ts">
  import { onMount } from "svelte";
  import { toast } from "svelte-sonner";
  import { UserPlus, Pencil, KeyRound, UserCheck, UserX } from "lucide-svelte";
  import {
    appSession,
    getUserRepository,
    hasPermission,
    PERMISSIONS,
    type RoleWithPermissions,
    type UserWithRole,
  } from "$lib/data";
  import { _ } from "$lib/i18n";
  import { Badge } from "$lib/ui/badge";
  import { Button } from "$lib/ui/button";
  import {
    Card,
    CardContent,
    CardDescription,
    CardHeader,
    CardTitle,
  } from "$lib/ui/card";
  import { Dialog } from "$lib/ui/dialog";
  import { Input } from "$lib/ui/input";
  import { EmptyState, ErrorState, LoadingState } from "$lib/ui";

  let isLoading = $state(true);
  let loadError = $state<string | null>(null);
  let users = $state<UserWithRole[]>([]);
  let roles = $state<RoleWithPermissions[]>([]);

  let canCreate = $derived(
    hasPermission(appSession.session, PERMISSIONS.USERS_CREATE),
  );
  let canEdit = $derived(
    hasPermission(appSession.session, PERMISSIONS.USERS_EDIT),
  );
  let canDeactivate = $derived(
    hasPermission(appSession.session, PERMISSIONS.USERS_DEACTIVATE),
  );
  let canResetPassword = $derived(
    hasPermission(appSession.session, PERMISSIONS.USERS_RESET_PASSWORD),
  );

  // Create modal state
  let createOpen = $state(false);
  let createName = $state("");
  let createEmail = $state("");
  let createPassword = $state("");
  let createRoleId = $state("");
  let isCreating = $state(false);
  let createError = $state<string | null>(null);

  // Edit modal state
  let editOpen = $state(false);
  let editingUser = $state<UserWithRole | null>(null);
  let editName = $state("");
  let editEmail = $state("");
  let editRoleId = $state("");
  let isEditing = $state(false);
  let editError = $state<string | null>(null);

  // Reset password modal state
  let resetOpen = $state(false);
  let resettingUser = $state<UserWithRole | null>(null);
  let resetNewPassword = $state("");
  let isResetting = $state(false);
  let resetError = $state<string | null>(null);

  async function loadData() {
    isLoading = true;
    loadError = null;
    try {
      const repo = getUserRepository();
      const [loadedUsers, loadedRoles] = await Promise.all([
        repo.listUsers(),
        repo.listRoles(),
      ]);
      users = loadedUsers;
      roles = loadedRoles;
      if (!createRoleId && loadedRoles.length > 0) {
        const comercial = loadedRoles.find((r) => r.slug === "comercial");
        createRoleId = (comercial ?? loadedRoles[0]).id;
      }
    } catch (err) {
      loadError = err instanceof Error ? err.message : String(err);
    } finally {
      isLoading = false;
    }
  }

  onMount(() => {
    loadData();
  });

  function openCreateModal() {
    createName = "";
    createEmail = "";
    createPassword = "";
    createError = null;
    const comercial = roles.find((r) => r.slug === "comercial");
    createRoleId = (comercial ?? roles[0])?.id ?? "";
    createOpen = true;
  }

  async function handleCreateUser(event: SubmitEvent) {
    event.preventDefault();
    createError = null;
    isCreating = true;

    try {
      const repo = getUserRepository();
      await repo.createUser({
        name: createName,
        email: createEmail,
        password: createPassword,
        role_id: createRoleId,
      });
      createOpen = false;
      toast.success($_("users.createSuccessToast"));
      await loadData();
    } catch (err) {
      createError = err instanceof Error ? err.message : String(err);
      toast.error(createError);
    } finally {
      isCreating = false;
    }
  }

  function openEditModal(user: UserWithRole) {
    editingUser = user;
    editName = user.name;
    editEmail = user.email;
    editRoleId = user.role_id;
    editError = null;
    editOpen = true;
  }

  async function handleUpdateUser(event: SubmitEvent) {
    event.preventDefault();
    if (!editingUser) return;
    editError = null;
    isEditing = true;

    try {
      const repo = getUserRepository();
      await repo.updateUser(editingUser.id, {
        name: editName,
        email: editEmail,
        role_id: editRoleId,
      });
      editOpen = false;
      toast.success($_("users.updateSuccessToast"));
      await loadData();
    } catch (err) {
      editError = err instanceof Error ? err.message : String(err);
      toast.error(editError);
    } finally {
      isEditing = false;
    }
  }

  async function handleToggleActive(user: UserWithRole) {
    try {
      const repo = getUserRepository();
      await repo.setUserActive(user.id, !user.active);
      toast.success($_("users.statusSuccessToast"));
      await loadData();
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      toast.error(msg);
    }
  }

  function openResetModal(user: UserWithRole) {
    resettingUser = user;
    resetNewPassword = "";
    resetError = null;
    resetOpen = true;
  }

  async function handleResetPassword(event: SubmitEvent) {
    event.preventDefault();
    if (!resettingUser) return;
    resetError = null;
    isResetting = true;

    try {
      const repo = getUserRepository();
      await repo.resetUserPassword(resettingUser.id, {
        new_password: resetNewPassword,
      });
      resetOpen = false;
      toast.success($_("users.resetSuccessToast"));
      await loadData();
    } catch (err) {
      resetError = err instanceof Error ? err.message : String(err);
      toast.error(resetError);
    } finally {
      isResetting = false;
    }
  }
</script>

<div class="space-y-6">
  <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
    <div>
      <h2 class="text-2xl font-bold tracking-tight text-foreground">
        {$_("users.title")}
      </h2>
      <p class="text-sm text-muted-foreground mt-1">
        {$_("users.description")}
      </p>
    </div>

    {#if canCreate}
      <Button onclick={openCreateModal} data-testid="new-user-btn">
        <UserPlus class="h-4 w-4" />
        {$_("users.newUserBtn")}
      </Button>
    {/if}
  </div>

  {#if isLoading}
    <LoadingState message={$_("users.loading")} />
  {:else if loadError}
    <ErrorState message={loadError} onRetry={loadData} />
  {:else if users.length === 0}
    <EmptyState />
  {:else}
    <Card>
      <CardHeader class="pb-3">
        <CardTitle class="text-base">{$_("users.title")}</CardTitle>
        <CardDescription>{$_("users.description")}</CardDescription>
      </CardHeader>
      <CardContent>
        <div class="overflow-x-auto">
          <table class="w-full text-left text-sm">
            <thead>
              <tr
                class="border-b border-border text-xs uppercase text-muted-foreground"
              >
                <th class="py-3 px-3 font-medium">{$_("users.tableName")}</th>
                <th class="py-3 px-3 font-medium">{$_("users.tableEmail")}</th>
                <th class="py-3 px-3 font-medium">{$_("users.tableRole")}</th>
                <th class="py-3 px-3 font-medium">{$_("users.tableStatus")}</th>
                <th class="py-3 px-3 text-right font-medium">
                  {$_("users.tableActions")}
                </th>
              </tr>
            </thead>
            <tbody class="divide-y divide-border">
              {#each users as user (user.id)}
                {@const isSelf = user.id === appSession.session?.user_id}
                <tr class="hover:bg-muted/30 transition-colors">
                  <td class="py-3.5 px-3 font-medium text-foreground">
                    <div class="flex items-center gap-2">
                      <span>{user.name}</span>
                      {#if isSelf}
                        <Badge variant="outline">{$_("users.youBadge")}</Badge>
                      {/if}
                    </div>
                  </td>
                  <td class="py-3.5 px-3 text-muted-foreground">
                    {user.email}
                  </td>
                  <td class="py-3.5 px-3">
                    <Badge
                      variant={user.role_slug === "proprietario"
                        ? "default"
                        : "secondary"}
                    >
                      {user.role_name}
                    </Badge>
                  </td>
                  <td class="py-3.5 px-3">
                    {#if !user.active}
                      <Badge variant="destructive">
                        {$_("users.statusInactive")}
                      </Badge>
                    {:else if user.locked_until && Date.parse(user.locked_until) > Date.now()}
                      <Badge variant="outline">
                        {$_("users.statusLocked")}
                      </Badge>
                    {:else}
                      <Badge variant="secondary">
                        {$_("users.statusActive")}
                      </Badge>
                    {/if}
                  </td>
                  <td class="py-3.5 px-3 text-right">
                    <div class="flex items-center justify-end gap-1.5">
                      {#if canEdit}
                        <Button
                          variant="ghost"
                          size="sm"
                          onclick={() => openEditModal(user)}
                          title={$_("users.editBtn")}
                        >
                          <Pencil class="h-3.5 w-3.5" />
                          <span class="sr-only sm:not-sr-only">
                            {$_("users.editBtn")}
                          </span>
                        </Button>
                      {/if}

                      {#if canResetPassword}
                        <Button
                          variant="ghost"
                          size="sm"
                          onclick={() => openResetModal(user)}
                          title={$_("users.resetPasswordBtn")}
                        >
                          <KeyRound class="h-3.5 w-3.5" />
                          <span class="sr-only lg:not-sr-only">
                            {$_("users.resetPasswordBtn")}
                          </span>
                        </Button>
                      {/if}

                      {#if canDeactivate && !isSelf}
                        <Button
                          variant="outline"
                          size="sm"
                          onclick={() => handleToggleActive(user)}
                        >
                          {#if user.active}
                            <UserX class="h-3.5 w-3.5 text-destructive" />
                            <span>{$_("users.deactivateBtn")}</span>
                          {:else}
                            <UserCheck class="h-3.5 w-3.5 text-success" />
                            <span>{$_("users.activateBtn")}</span>
                          {/if}
                        </Button>
                      {/if}
                    </div>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </CardContent>
    </Card>
  {/if}
</div>

<Dialog
  bind:open={createOpen}
  title={$_("users.createModalTitle")}
  description={$_("users.createModalDesc")}
>
  <form onsubmit={handleCreateUser} class="space-y-4">
    {#if createError}
      <div
        role="alert"
        class="rounded-[var(--radius)] border border-destructive/40 bg-destructive/10 px-3 py-2 text-xs font-medium text-destructive"
      >
        {createError}
      </div>
    {/if}

    <div class="space-y-2">
      <label for="create-user-name" class="text-sm font-medium text-foreground">
        {$_("users.nameLabel")}
      </label>
      <Input
        id="create-user-name"
        bind:value={createName}
        placeholder={$_("users.namePlaceholder")}
        required
      />
    </div>

    <div class="space-y-2">
      <label
        for="create-user-email"
        class="text-sm font-medium text-foreground"
      >
        {$_("users.emailLabel")}
      </label>
      <Input
        id="create-user-email"
        type="email"
        bind:value={createEmail}
        placeholder={$_("users.emailPlaceholder")}
        required
      />
    </div>

    <div class="space-y-2">
      <label
        for="create-user-password"
        class="text-sm font-medium text-foreground"
      >
        {$_("users.passwordLabel")}
      </label>
      <Input
        id="create-user-password"
        type="password"
        bind:value={createPassword}
        placeholder={$_("users.passwordPlaceholder")}
        minlength={8}
        required
      />
    </div>

    <div class="space-y-2">
      <label for="create-user-role" class="text-sm font-medium text-foreground">
        {$_("users.roleLabel")}
      </label>
      <select
        id="create-user-role"
        bind:value={createRoleId}
        class="flex h-10 w-full rounded-[var(--radius)] border border-input bg-background px-3 py-2 text-sm text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        required
      >
        {#each roles as role (role.id)}
          <option value={role.id}>{role.name}</option>
        {/each}
      </select>
    </div>

    <div class="flex justify-end gap-2 pt-2">
      <Button
        type="button"
        variant="ghost"
        onclick={() => (createOpen = false)}
      >
        {$_("users.cancelBtn")}
      </Button>
      <Button type="submit" disabled={isCreating}>
        {isCreating ? $_("users.saving") : $_("users.saveBtn")}
      </Button>
    </div>
  </form>
</Dialog>

<Dialog
  bind:open={editOpen}
  title={$_("users.editModalTitle")}
  description={$_("users.editModalDesc")}
>
  <form onsubmit={handleUpdateUser} class="space-y-4">
    {#if editError}
      <div
        role="alert"
        class="rounded-[var(--radius)] border border-destructive/40 bg-destructive/10 px-3 py-2 text-xs font-medium text-destructive"
      >
        {editError}
      </div>
    {/if}

    <div class="space-y-2">
      <label for="edit-user-name" class="text-sm font-medium text-foreground">
        {$_("users.nameLabel")}
      </label>
      <Input
        id="edit-user-name"
        bind:value={editName}
        placeholder={$_("users.namePlaceholder")}
        required
      />
    </div>

    <div class="space-y-2">
      <label for="edit-user-email" class="text-sm font-medium text-foreground">
        {$_("users.emailLabel")}
      </label>
      <Input
        id="edit-user-email"
        type="email"
        bind:value={editEmail}
        placeholder={$_("users.emailPlaceholder")}
        required
      />
    </div>

    <div class="space-y-2">
      <label for="edit-user-role" class="text-sm font-medium text-foreground">
        {$_("users.roleLabel")}
      </label>
      <select
        id="edit-user-role"
        bind:value={editRoleId}
        class="flex h-10 w-full rounded-[var(--radius)] border border-input bg-background px-3 py-2 text-sm text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        required
      >
        {#each roles as role (role.id)}
          <option value={role.id}>{role.name}</option>
        {/each}
      </select>
    </div>

    <div class="flex justify-end gap-2 pt-2">
      <Button type="button" variant="ghost" onclick={() => (editOpen = false)}>
        {$_("users.cancelBtn")}
      </Button>
      <Button type="submit" disabled={isEditing}>
        {isEditing ? $_("users.saving") : $_("users.saveBtn")}
      </Button>
    </div>
  </form>
</Dialog>

<Dialog
  bind:open={resetOpen}
  title={$_("users.resetModalTitle")}
  description={$_("users.resetModalDesc")}
>
  <form onsubmit={handleResetPassword} class="space-y-4">
    {#if resetError}
      <div
        role="alert"
        class="rounded-[var(--radius)] border border-destructive/40 bg-destructive/10 px-3 py-2 text-xs font-medium text-destructive"
      >
        {resetError}
      </div>
    {/if}

    <div class="space-y-2">
      <label
        for="reset-user-password"
        class="text-sm font-medium text-foreground"
      >
        {$_("users.newPasswordLabel")}
      </label>
      <Input
        id="reset-user-password"
        type="password"
        bind:value={resetNewPassword}
        placeholder={$_("users.passwordPlaceholder")}
        minlength={8}
        required
      />
    </div>

    <div class="flex justify-end gap-2 pt-2">
      <Button type="button" variant="ghost" onclick={() => (resetOpen = false)}>
        {$_("users.cancelBtn")}
      </Button>
      <Button type="submit" disabled={isResetting}>
        {isResetting ? $_("users.saving") : $_("users.saveBtn")}
      </Button>
    </div>
  </form>
</Dialog>
