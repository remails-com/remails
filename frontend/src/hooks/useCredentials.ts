import { RemailsError } from "../error/error.ts";
import { useSelector } from "./useSelector.ts";

export function useCredentials() {
  const credentials = useSelector((state) => state.credentials || []);
  const routerState = useSelector((state) => state.routerState);
  const currentCredential = credentials?.find((s) => s.id === routerState.params.credential_id) || null;

  if (!currentCredential && routerState.params.credential_id) {
    throw new RemailsError(`Could not find credential with ID ${routerState.params.credential_id}`, 404);
  }

  return { credentials, currentCredential };
}
