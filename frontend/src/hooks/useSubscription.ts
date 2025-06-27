import { useOrganizations } from "./useOrganizations.ts";
import { SubscriptionStatus } from "../types.ts";
import { useEffect, useState } from "react";

export function useSubscription() {
  const { currentOrganization } = useOrganizations();
  const [subscription, setSubscription] = useState<SubscriptionStatus | null>(null);
  const [salesLink, setSalesLink] = useState<string | null>(null);

  useEffect(() => {
    if (currentOrganization) {
      fetch(`/api/organizations/${currentOrganization.id}/subscription`)
        .then((res) => res.json())
        .then(setSubscription);
    }
  }, [currentOrganization]);

  useEffect(() => {
    if (currentOrganization && subscription && subscription.status === "none") {
      fetch(`/api/organizations/${currentOrganization.id}/subscription/new`)
        .then((res) => res.json())
        .then(setSalesLink);
    }
  }, [currentOrganization, subscription]);

  return { subscription, salesLink };
}
