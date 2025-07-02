import { useRemails } from "./useRemails.ts";

export function useDomains(projectDomains = false) {
  const {
    state: { domains, organizationDomains, routerState },
  } = useRemails();

  const selectedDomains = projectDomains ? domains : organizationDomains;
  const currentDomain = selectedDomains?.find((d) => d.id === routerState.params.domain_id) || null;

  return { domains: selectedDomains, currentDomain };
}
