<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { ArrowLeft, Pencil, Trash2 } from "lucide-svelte";
  import { toast } from "svelte-sonner";
  import {
    appSession,
    getCustomerRepository,
    hasPermission,
    PERMISSIONS,
    type CreateCustomerInput,
    type CustomerWithDetails,
  } from "$lib/data";
  import { _ } from "$lib/i18n";
  import { formatDocument } from "$lib/utils/document";
  import CustomerForm from "$lib/components/customer-form.svelte";
  import { Badge } from "$lib/ui/badge";
  import { Button } from "$lib/ui/button";
  import { Dialog } from "$lib/ui/dialog";
  import { ErrorState, LoadingState } from "$lib/ui";

  let customerId = $derived(page.params.id ?? "");
  let isLoading = $state(true);
  let loadError = $state<string | null>(null);
  let details = $state<CustomerWithDetails | null>(null);

  let isEditing = $state(false);
  let submitting = $state(false);
  let submitError = $state<string | null>(null);

  let deleteOpen = $state(false);
  let isDeleting = $state(false);

  let canManage = $derived(
    hasPermission(appSession.session, PERMISSIONS.CUSTOMER_MANAGE),
  );

  async function loadCustomer() {
    if (!customerId) return;
    isLoading = true;
    loadError = null;
    try {
      details = await getCustomerRepository().getCustomer(customerId);
    } catch (err) {
      loadError = err instanceof Error ? err.message : String(err);
    } finally {
      isLoading = false;
    }
  }

  onMount(() => {
    loadCustomer();
  });

  function toFormInput(data: CustomerWithDetails): CreateCustomerInput {
    return {
      person_type: data.customer.person_type,
      name: data.customer.name,
      trade_name: data.customer.trade_name ?? "",
      document_number: data.customer.document_number,
      state_registration: data.customer.state_registration ?? "",
      municipal_registration: data.customer.municipal_registration ?? "",
      notes: data.customer.notes ?? "",
      contacts: data.contacts.map((c) => ({
        name: c.name,
        email: c.email ?? "",
        phone: c.phone ?? "",
        role: c.role ?? "",
        is_primary: c.is_primary,
      })),
      addresses: data.addresses.map((a) => ({
        label: a.label,
        street: a.street,
        number: a.number,
        complement: a.complement ?? "",
        neighborhood: a.neighborhood,
        city: a.city,
        state: a.state,
        postal_code: a.postal_code,
        is_primary: a.is_primary,
      })),
    };
  }

  async function handleUpdate(input: CreateCustomerInput) {
    if (!details) return;
    submitting = true;
    submitError = null;
    try {
      const updated = await getCustomerRepository().updateCustomer(
        details.customer.id,
        input,
      );
      details = updated;
      isEditing = false;
      toast.success($_("customers.updateSuccessToast"));
    } catch (err) {
      submitError = err instanceof Error ? err.message : String(err);
      toast.error(submitError);
    } finally {
      submitting = false;
    }
  }

  async function handleConfirmDelete() {
    if (!details) return;
    isDeleting = true;
    try {
      await getCustomerRepository().deleteCustomer(details.customer.id);
      deleteOpen = false;
      toast.success($_("customers.deleteSuccessToast"));
      await goto("/customers");
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      toast.error(msg);
    } finally {
      isDeleting = false;
    }
  }
</script>

<div class="space-y-6">
  <div class="flex flex-wrap items-center justify-between gap-3">
    <Button
      variant="ghost"
      size="sm"
      onclick={() => goto("/customers")}
      data-testid="back-to-customers-btn"
    >
      <ArrowLeft class="h-4 w-4" />
      {$_("customers.backToList")}
    </Button>

    {#if details && canManage}
      <div class="flex items-center gap-2">
        {#if !isEditing}
          <Button
            variant="outline"
            size="sm"
            onclick={() => {
              submitError = null;
              isEditing = true;
            }}
            data-testid="edit-customer-btn"
          >
            <Pencil class="h-4 w-4" />
            {$_("customers.editBtn")}
          </Button>
        {/if}
        <Button
          variant="destructive"
          size="sm"
          onclick={() => (deleteOpen = true)}
          data-testid="delete-customer-btn"
        >
          <Trash2 class="h-4 w-4" />
          {$_("customers.deleteBtn")}
        </Button>
      </div>
    {/if}
  </div>

  {#if isLoading}
    <LoadingState message={$_("customers.loading")} />
  {:else if loadError}
    <ErrorState message={loadError} onRetry={loadCustomer} />
  {:else if details}
    <div class="space-y-2">
      <div class="flex flex-wrap items-center gap-3">
        <h2 class="text-2xl font-bold tracking-tight text-foreground">
          {details.customer.name}
        </h2>
        <Badge variant="secondary">
          {details.customer.person_type === "legal"
            ? $_("customers.personTypeLegalFull")
            : $_("customers.personTypeIndividualFull")}
        </Badge>
      </div>
      <p class="text-sm text-muted-foreground">
        {formatDocument(
          details.customer.document_number,
          details.customer.person_type,
        )}
        {#if details.customer.trade_name}
          <span> • {details.customer.trade_name}</span>
        {/if}
      </p>
      {#if !canManage}
        <p class="text-xs text-muted-foreground">
          {$_("customers.readonlyNotice")}
        </p>
      {/if}
    </div>

    {#key isEditing ? "editing" : details.customer.updated_at}
      <CustomerForm
        initialValues={toFormInput(details)}
        readonly={!isEditing || !canManage}
        {submitting}
        {submitError}
        onSubmit={handleUpdate}
        onCancel={() => {
          submitError = null;
          isEditing = false;
        }}
      />
    {/key}
  {/if}
</div>

<Dialog
  bind:open={deleteOpen}
  title={$_("customers.deleteConfirmTitle")}
  description={details
    ? $_("customers.deleteConfirmDesc", {
        values: { name: details.customer.name },
      })
    : ""}
>
  <div class="flex justify-end gap-2 pt-2">
    <Button
      type="button"
      variant="ghost"
      onclick={() => (deleteOpen = false)}
      disabled={isDeleting}
    >
      {$_("customers.cancelBtn")}
    </Button>
    <Button
      type="button"
      variant="destructive"
      onclick={handleConfirmDelete}
      disabled={isDeleting}
      data-testid="confirm-delete-customer-btn"
    >
      {isDeleting ? $_("customers.saving") : $_("customers.deleteConfirmBtn")}
    </Button>
  </div>
</Dialog>
