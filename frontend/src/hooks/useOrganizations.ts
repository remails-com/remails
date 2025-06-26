import { useRemails } from "./useRemails.ts";

export function useOrganizations() {
  const {
    state: { organizations, routerState },
  } = useRemails();
  const currentOrganization = organizations?.find((o) => o.id === routerState.params.org_id) || null;

  return { organizations, currentOrganization };
}
