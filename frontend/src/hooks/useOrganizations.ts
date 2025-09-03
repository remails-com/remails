import { useSelector } from "./useSelector";
import { Invite, OrganizationMember } from "../types.ts";
import { useEffect, useState } from "react";
import { errorNotification } from "../notify.tsx";

export function useOrganizations() {
  const organizations = useSelector((state) => state.organizations || []);
  const routerState = useSelector((state) => state.routerState);
  const currentOrganization = organizations.find((o) => o.id === routerState.params.org_id);

  return { organizations, currentOrganization };
}

export function useInvites() {
  const { currentOrganization } = useOrganizations();
  const [invites, setInvites] = useState<Invite[] | null>(null);

  useEffect(() => {
    if (currentOrganization) {
      fetch(`/api/invite/${currentOrganization.id}`)
        .then((res) => {
          if (res.status === 200) {
            return res.json();
          } else {
            errorNotification("Failed to load the organization's invites");
            console.error(res);
            return null;
          }
        })
        .then(setInvites);
    }
  }, [currentOrganization]);

  return { invites, setInvites };
}

export function useMembers() {
  const { currentOrganization } = useOrganizations();
  const [members, setMembers] = useState<OrganizationMember[] | null>(null);

  useEffect(() => {
    if (currentOrganization) {
      fetch(`/api/organizations/${currentOrganization.id}/members`)
        .then((res) => {
          if (res.status === 200) {
            return res.json();
          } else {
            errorNotification("Failed to load the organization's menbers");
            console.error(res);
            return null;
          }
        })
        .then(setMembers);
    }
  }, [currentOrganization]);

  return { members, setMembers };
}
