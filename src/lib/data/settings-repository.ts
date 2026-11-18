import type {
  AllocationSettings,
  UpdateAllocationSettingsInput,
} from "./types";

export interface SettingsRepository {
  getAllocationSettings(): Promise<AllocationSettings>;
  updateAllocationSettings(
    input: UpdateAllocationSettingsInput,
  ): Promise<AllocationSettings>;
}
