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
            errorNotification("Failed to load the organization's members");
            console.error(res);
            return null;
          }
        })
        .then(setMembers);
    }
  }, [currentOrganization]);

  return { members, setMembers };
}

export function useStatistics() {
  const statistics = useSelector((state) => state.statistics);

  return { monthly_statistics: statistics.monthly, daily_statistics: statistics.daily };
}

export function useOrgRole() {
  const { currentOrganization } = useOrganizations();
  const user = useSelector((state) => state.user);

  const isAdmin =
    user.org_roles.some((o) => o.org_id == currentOrganization?.id && o.role == "admin") || user.global_role == "admin";

  const isMaintainer =
    isAdmin || user.org_roles.some((o) => o.org_id == currentOrganization?.id && o.role == "maintainer");

  return { isAdmin, isMaintainer };
}
