import type {
  CreateCustomerInput,
  CustomerFilter,
  CustomerPage,
  CustomerWithDetails,
  UpdateCustomerInput,
} from "./types";

export interface CustomerRepository {
  listCustomers(filter: CustomerFilter): Promise<CustomerPage>;
  getCustomer(id: string): Promise<CustomerWithDetails>;
  createCustomer(input: CreateCustomerInput): Promise<CustomerWithDetails>;
  updateCustomer(
    id: string,
    input: UpdateCustomerInput,
  ): Promise<CustomerWithDetails>;
  deleteCustomer(id: string): Promise<void>;
}
