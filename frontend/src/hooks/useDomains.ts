import { useRemails } from "./useRemails.ts";

export function useDomains() {
  const {
    state: { domains, organizationDomains, routerState },
    navigate,
  } = useRemails();

  const selectedDomains = routerState.params.proj_id ? domains : organizationDomains;
  const currentDomain = selectedDomains?.find((d) => d.id === routerState.params.domain_id) || null;

  if (!currentDomain && routerState.params.domain_id) {
    console.error("Domain not found", selectedDomains, currentDomain, routerState.params);
    navigate("not_found");
  }

  return { domains: selectedDomains, currentDomain };
}
