import {useRemails} from "./useRemails.ts";

export function useDomains() {
  const {state: {domains, pathParams}} = useRemails();
  const currentDomain = domains?.find((d) => d.id === pathParams.domain_id) || null;

  return {domains, currentDomain}
}
