import {useRemails} from "./useRemails.ts";

export function useCurrentOrganisation() {
  const {state: {organizations, pathParams}} = useRemails();
  return organizations?.find((o) => o.id === pathParams.org_id) || null;
}
