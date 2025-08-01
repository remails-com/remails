import { useOrganizations } from "./useOrganizations.ts";
import { SubscriptionStatus } from "../types.ts";
import { useEffect, useState } from "react";
import { errorNotification } from "../notify.tsx";

export function useSubscription() {
  const { currentOrganization } = useOrganizations();
  const [subscription, setSubscription] = useState<SubscriptionStatus | null>(null);
  const [salesLink, setSalesLink] = useState<string | null>(null);

  useEffect(() => {
    if (currentOrganization) {
      fetch(`/api/organizations/${currentOrganization.id}/subscription`)
        .then((res) => {
          if (res.status === 200) {
            return res.json();
          } else {
            errorNotification("Failed to load the current subscription");
            console.error(res);
            return null;
          }
        })
        .then(setSubscription);
    }
  }, [currentOrganization]);

  useEffect(() => {
    if (currentOrganization && subscription && subscription.status === "none") {
      fetch(`/api/organizations/${currentOrganization.id}/subscription/new`)
        .then((res) => {
          if (res.status === 200) {
            return res.json();
          } else {
            return null;
          }
        })
        .then(setSalesLink);
    }
  }, [currentOrganization, subscription]);

  return { subscription, salesLink };
}
