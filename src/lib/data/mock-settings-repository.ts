import type { SettingsRepository } from "./settings-repository";
import type {
  AllocationSettings,
  UpdateAllocationSettingsInput,
} from "./types";

if (!import.meta.env.DEV) {
  throw new Error(
    "Mocks não podem ser carregados no ambiente de produção. Execute o aplicativo desktop EventRent.",
  );
}

const STORAGE_KEY = "eventrent:mock:allocation_settings:v2";

const DEFAULT_SETTINGS: AllocationSettings = {
  changeover_minutes: 30,
  internal_transfer_minutes: 20,
  external_transfer_minutes: 60,
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
        changeover_minutes:
          typeof parsed.changeover_minutes === "number"
            ? parsed.changeover_minutes
            : DEFAULT_SETTINGS.changeover_minutes,
        internal_transfer_minutes:
          typeof parsed.internal_transfer_minutes === "number"
            ? parsed.internal_transfer_minutes
            : DEFAULT_SETTINGS.internal_transfer_minutes,
        external_transfer_minutes:
          typeof parsed.external_transfer_minutes === "number"
            ? parsed.external_transfer_minutes
            : DEFAULT_SETTINGS.external_transfer_minutes,
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
      {
        label: "Virada no mesmo espaço",
        value: input.changeover_minutes,
        max: 240,
      },
      {
        label: "Deslocamento interno",
        value: input.internal_transfer_minutes,
        max: 240,
      },
      {
        label: "Deslocamento externo",
        value: input.external_transfer_minutes,
        max: 720,
      },
    ];
    for (const f of fields) {
      if (!Number.isInteger(f.value) || f.value < 0 || f.value > f.max) {
        throw new Error(`${f.label} deve estar entre 0 e ${f.max} minutos.`);
      }
    }
    const next: AllocationSettings = {
      changeover_minutes: input.changeover_minutes,
      internal_transfer_minutes: input.internal_transfer_minutes,
      external_transfer_minutes: input.external_transfer_minutes,
    };
    if (typeof window !== "undefined") {
      window.localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
    }
    return next;
  }
}
