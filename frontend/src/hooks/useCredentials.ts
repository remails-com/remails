import { useRemails } from "./useRemails.ts";

export function useCredentials() {
  const {
    state: { credentials, routerState },
    navigate,
  } = useRemails();
  const currentCredential = credentials?.find((s) => s.id === routerState.params.credential_id) || null;

  if (!currentCredential && routerState.params.credential_id) {
    navigate("not_found");
  }

  return { credentials, currentCredential };
}
