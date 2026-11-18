import { invoke } from "@tauri-apps/api/core";
import type { SettingsRepository } from "./settings-repository";
import type {
  AllocationSettings,
  UpdateAllocationSettingsInput,
} from "./types";

export class TauriSettingsRepository implements SettingsRepository {
  async getAllocationSettings(): Promise<AllocationSettings> {
    return invoke<AllocationSettings>("get_allocation_settings");
  }

  async updateAllocationSettings(
    input: UpdateAllocationSettingsInput,
  ): Promise<AllocationSettings> {
    return invoke<AllocationSettings>("update_allocation_settings", { input });
  }
}
