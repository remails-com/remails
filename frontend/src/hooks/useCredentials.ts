import { useRemails } from "./useRemails.ts";

export function useCredentials() {
  const {
    state: { credentials, routerState },
  } = useRemails();
  const currentCredential = credentials?.find((s) => s.id === routerState.params.credential_id) || null;

  return { credentials, currentCredential };
}
