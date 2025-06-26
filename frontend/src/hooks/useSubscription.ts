import { useOrganizations } from "./useOrganizations.ts";
import { SubscriptionStatus } from "../types.ts";
import { useEffect, useState } from "react";

export function useSubscription() {
  const { currentOrganization } = useOrganizations();
  const [subscription, setSubscription] = useState<SubscriptionStatus | null>(null);

  useEffect(() => {
    if (currentOrganization) {
      fetch(`/api/organizations/${currentOrganization.id}/subscription`)
        .then((res) => res.json())
        .then(setSubscription);
    }
  }, [currentOrganization]);

  return { subscription };
}