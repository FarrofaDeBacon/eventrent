import type { HandleClientError } from "@sveltejs/kit";

export const handleError: HandleClientError = ({ error, status, message }) => {
  console.error("SvelteKit client error:", { error, status, message });
  return {
    message: error instanceof Error ? error.message : message,
    stack: error instanceof Error ? error.stack : undefined,
  };
};
