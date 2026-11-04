import { init, register, waitLocale } from "svelte-i18n";

register("pt-BR", () => import("./locales/pt-BR.json"));

export function setupI18n(): void {
  init({
    fallbackLocale: "pt-BR",
    initialLocale: "pt-BR",
  });
}

export { waitLocale };
export { _, t, locale, locales } from "svelte-i18n";
