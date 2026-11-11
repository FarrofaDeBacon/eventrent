import { invoke } from "@tauri-apps/api/core";
import type { CustomerRepository } from "./customer-repository";
import type {
  CreateCustomerInput,
  CustomerFilter,
  CustomerPage,
  CustomerWithDetails,
  UpdateCustomerInput,
} from "./types";

export class TauriCustomerRepository implements CustomerRepository {
  async listCustomers(filter: CustomerFilter): Promise<CustomerPage> {
    return await invoke<CustomerPage>("list_customers", { filter });
  }

  async getCustomer(id: string): Promise<CustomerWithDetails> {
    return await invoke<CustomerWithDetails>("get_customer", { id });
  }

  async createCustomer(
    input: CreateCustomerInput,
  ): Promise<CustomerWithDetails> {
    return await invoke<CustomerWithDetails>("create_customer", { input });
  }

  async updateCustomer(
    id: string,
    input: UpdateCustomerInput,
  ): Promise<CustomerWithDetails> {
    return await invoke<CustomerWithDetails>("update_customer", { id, input });
  }

  async deleteCustomer(id: string): Promise<void> {
    await invoke<void>("delete_customer", { id });
  }
}
