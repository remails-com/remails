import { useRemails } from "./useRemails.ts";

export function useDomains() {
  const {
    state: { domains, routerState },
  } = useRemails();
  const currentDomain = domains?.find((d) => d.id === routerState.params.domain_id) || null;

  return { domains, currentDomain };
}
