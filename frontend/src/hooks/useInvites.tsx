import { useOrganizations } from "./useOrganizations.ts";
import { Invite } from "../types.ts";
import { useEffect, useState } from "react";
import { errorNotification } from "../notify.tsx";

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
