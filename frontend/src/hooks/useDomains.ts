import { useRemails } from "./useRemails.ts";

export function useDomains(projectDomains = false) {
  const {
    state: { domains, organizationDomains, routerState },
    navigate,
  } = useRemails();

  const selectedDomains = projectDomains ? domains : organizationDomains;
  const currentDomain = selectedDomains?.find((d) => d.id === routerState.params.domain_id) || null;

  if (!currentDomain && routerState.params.domain_id) {
    navigate("not_found");
  }

  return { domains: selectedDomains, currentDomain };
}
