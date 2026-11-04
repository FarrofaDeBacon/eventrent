import { addMessages, init, waitLocale } from "svelte-i18n";
import ptBR from "./locales/pt-BR.json";

let initialized = false;

export function setupI18n(): void {
  if (initialized) return;
  addMessages("pt-BR", ptBR);
  init({
    fallbackLocale: "pt-BR",
    initialLocale: "pt-BR",
  });
  initialized = true;
}

setupI18n();

export { waitLocale };
export { _, t, locale, locales } from "svelte-i18n";
