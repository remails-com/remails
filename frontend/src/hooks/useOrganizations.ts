import { useSelector } from "./useSelector";

export function useOrganizations() {
  const organizations = useSelector((state) => state.organizations || []);
  const routerState = useSelector((state) => state.routerState);
  const currentOrganization = organizations.find((o) => o.id === routerState.params.org_id);

  return { organizations, currentOrganization };
}
