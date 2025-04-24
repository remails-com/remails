import {useRemails} from "./useRemails.ts";

export function useCurrentOrganisation() {
  const {state: {organizations, params}} = useRemails();
  const currentOrganisation = organizations?.find((o) => o.id === params.org_id) || null;

  return currentOrganisation;
}
