import { useOrganizations } from "./useOrganizations.ts";
import { SubscriptionStatus } from "../types.ts";
import { useEffect, useState } from "react";
import { notifications } from "@mantine/notifications";
import { IconX } from "@tabler/icons-react";

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
            notifications.show({
              title: "Error",
              message: "Failed to load the current subscription",
              color: "red",
              autoClose: 20000,
              icon: <IconX size={20} />,
            });
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
