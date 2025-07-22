import { RemailsError } from "../error/error.ts";
import { useSelector } from "./useSelector.ts";

export function useDomains() {
  const domains = useSelector((state) => state.domains || []);
  const organizationDomains = useSelector((state) => state.organizationDomains || []);
  const routerState = useSelector((state) => state.routerState);
  const selectedDomains = routerState.params.proj_id ? domains : organizationDomains;
  const currentDomain = selectedDomains?.find((d) => d.id === routerState.params.domain_id) || null;

  if (!currentDomain && routerState.params.domain_id) {
    throw new RemailsError(`Could not find domain with ID ${routerState.params.domain_id}`, 404);
  }

  return { domains: selectedDomains, currentDomain };
}
