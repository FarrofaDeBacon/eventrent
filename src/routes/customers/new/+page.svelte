<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { ArrowLeft } from "lucide-svelte";
  import { toast } from "svelte-sonner";
  import {
    appSession,
    getCustomerRepository,
    hasPermission,
    PERMISSIONS,
    type CreateCustomerInput,
  } from "$lib/data";
  import { _ } from "$lib/i18n";
  import CustomerForm from "$lib/components/customer-form.svelte";
  import { Button } from "$lib/ui/button";

  let submitting = $state(false);
  let submitError = $state<string | null>(null);

  let canManageCustomer = $derived(
    hasPermission(appSession.session, PERMISSIONS.CUSTOMER_MANAGE),
  );
  let canManageSupplier = $derived(
    hasPermission(appSession.session, PERMISSIONS.SUPPLIER_MANAGE),
  );

  let isSupplierContext = $derived(
    page.url.searchParams.get("role") === "supplier",
  );
  let defaultIsSupplier = $derived(
    isSupplierContext
      ? canManageSupplier
      : !canManageCustomer && canManageSupplier,
  );
  let defaultIsCustomer = $derived(
    isSupplierContext ? false : canManageCustomer,
  );

  async function handleCreate(input: CreateCustomerInput) {
    submitting = true;
    submitError = null;
    try {
      const created = await getCustomerRepository().createCustomer(input);
      toast.success($_("customers.createSuccessToast"));
      await goto(`/customers/${created.customer.id}`);
    } catch (err) {
      submitError = err instanceof Error ? err.message : String(err);
      toast.error(submitError);
    } finally {
      submitting = false;
    }
  }
</script>

<div class="space-y-6">
  <div class="flex items-center gap-3">
    <Button
      variant="ghost"
      size="sm"
      onclick={() => goto("/customers")}
      data-testid="back-to-customers-btn"
    >
      <ArrowLeft class="h-4 w-4" />
      {$_("customers.backToList")}
    </Button>
  </div>

  <div>
    <h2 class="text-2xl font-bold tracking-tight text-foreground">
      {$_("customers.newTitle")}
    </h2>
    <p class="text-sm text-muted-foreground mt-1">
      {$_("customers.newDescription")}
    </p>
  </div>

  <CustomerForm
    {defaultIsCustomer}
    {defaultIsSupplier}
    {canManageCustomer}
    {canManageSupplier}
    {submitting}
    {submitError}
    onSubmit={handleCreate}
    onCancel={() => goto(isSupplierContext ? "/suppliers" : "/customers")}
  />
</div>
