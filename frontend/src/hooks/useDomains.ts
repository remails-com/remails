import { RemailsError } from "../error/error.ts";
import { useSelector } from "./useSelector.ts";

export function useDomains() {
  const domains = useSelector((state) => state.domains || []);
  const routerState = useSelector((state) => state.routerState);
  const currentDomain = domains.find((d) => d.id === routerState.params.domain_id) ?? null;

  if (!currentDomain && routerState.params.domain_id) {
    throw new RemailsError(`Could not find domain with ID ${routerState.params.domain_id}`, 404);
  }

  return { domains, currentDomain };
}
