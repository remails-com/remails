import { useRemails } from "./useRemails.ts";

export function useCredentials() {
  const {
    state: { credentials, pathParams },
  } = useRemails();
  const currentCredential = credentials?.find((s) => s.id === pathParams.credential_id) || null;

  return { credentials, currentCredential };
}
