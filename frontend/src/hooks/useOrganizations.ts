import {useRemails} from "./useRemails.ts";

export function useOrganizations() {
  const {state: {organizations, pathParams}} = useRemails();
  const currentOrganization =  organizations?.find((o) => o.id === pathParams.org_id) || null;

  return {organizations, currentOrganization}
}
