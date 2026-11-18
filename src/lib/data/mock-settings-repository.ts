import type { SettingsRepository } from "./settings-repository";
import type {
  AllocationSettings,
  UpdateAllocationSettingsInput,
} from "./types";

const STORAGE_KEY = "eventrent:mock:allocation_settings:v1";

const DEFAULT_SETTINGS: AllocationSettings = {
  default_setup_minutes: 30,
  default_teardown_minutes: 20,
  default_turnover_buffer_minutes: 60,
};

export class MockSettingsRepository implements SettingsRepository {
  private load(): AllocationSettings {
    if (typeof window === "undefined") {
      return { ...DEFAULT_SETTINGS };
    }
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) {
      return { ...DEFAULT_SETTINGS };
    }
    try {
      const parsed = JSON.parse(raw) as Partial<AllocationSettings>;
      return {
        default_setup_minutes:
          typeof parsed.default_setup_minutes === "number"
            ? parsed.default_setup_minutes
            : DEFAULT_SETTINGS.default_setup_minutes,
        default_teardown_minutes:
          typeof parsed.default_teardown_minutes === "number"
            ? parsed.default_teardown_minutes
            : DEFAULT_SETTINGS.default_teardown_minutes,
        default_turnover_buffer_minutes:
          typeof parsed.default_turnover_buffer_minutes === "number"
            ? parsed.default_turnover_buffer_minutes
            : DEFAULT_SETTINGS.default_turnover_buffer_minutes,
      };
    } catch {
      return { ...DEFAULT_SETTINGS };
    }
  }

  async getAllocationSettings(): Promise<AllocationSettings> {
    return this.load();
  }

  async updateAllocationSettings(
    input: UpdateAllocationSettingsInput,
  ): Promise<AllocationSettings> {
    const fields = [
      { label: "Tempo padrão de montagem", value: input.default_setup_minutes },
      {
        label: "Tempo padrão de desmontagem",
        value: input.default_teardown_minutes,
      },
      {
        label: "Buffer padrão de giro",
        value: input.default_turnover_buffer_minutes,
      },
    ];
    for (const f of fields) {
      if (!Number.isInteger(f.value) || f.value < 0 || f.value > 240) {
        throw new Error(`${f.label} deve estar entre 0 e 240 minutos.`);
      }
    }
    const next: AllocationSettings = {
      default_setup_minutes: input.default_setup_minutes,
      default_teardown_minutes: input.default_teardown_minutes,
      default_turnover_buffer_minutes: input.default_turnover_buffer_minutes,
    };
    if (typeof window !== "undefined") {
      window.localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
    }
    return next;
  }
}
