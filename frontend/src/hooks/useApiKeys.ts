import { RemailsError } from "../error/error.ts";
import { useSelector } from "./useSelector.ts";

export function useApiKeys() {
  const apiKeys = useSelector((state) => state.apiKeys || []);
  const routerState = useSelector((state) => state.routerState);
  const currentApiKey = apiKeys?.find((s) => s.id === routerState.params.api_key_id) || null;

  if (!currentApiKey && routerState.params.api_key_id) {
    throw new RemailsError(`Could not find API key with ID ${routerState.params.api_key_id}`, 404);
  }

  return { apiKeys, currentApiKey };
}
