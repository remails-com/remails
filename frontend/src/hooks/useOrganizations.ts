import { useRemails } from "./useRemails.ts";

export function useOrganizations() {
  const {
    state: { organizations, routerState },
    navigate,
  } = useRemails();
  const currentOrganization = organizations?.find((o) => o.id === routerState.params.org_id) || null;

  if (!currentOrganization && routerState.params.org_id) {
    console.error("Organization not found");
    navigate("not_found");
  }

  return { organizations, currentOrganization };
}
