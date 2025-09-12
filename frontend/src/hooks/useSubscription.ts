import { useOrganizations } from "./useOrganizations.ts";
import { useRemails } from "./useRemails.ts";
import { errorNotification } from "../notify.tsx";

export function useSubscription() {
  const { currentOrganization } = useOrganizations();
  const {
    dispatch,
    navigate,
    state: { routerState },
  } = useRemails();

  const generateSalesLink = async (): Promise<string | null> => {
    if (!currentOrganization) {
      return null;
    }
    const res = await fetch(`/api/organizations/${currentOrganization.id}/subscription/new`);
    if (res.status === 200) {
      return res.json();
    } else {
      return null;
    }
  };

  const navigateToSales = async () => {
    const link = await generateSalesLink();
    if (!link) {
      console.error("Could not generate sales link");
      errorNotification("Can't connect to sales backend, try again later");
      return;
    }
    window.open(link, "_blank")!.focus();
  };

  const reloadSubscription = async () => {
    if (!currentOrganization) {
      console.error("Cannot reload subscription status without an organization");
      return;
    }
    const res = await fetch(`/api/organizations/${currentOrganization.id}/subscription`);
    if (res.status !== 200) {
      errorNotification("Something went wrong while reloading the subscription status. Please try again later.");
      console.error(`Failed to reload subscription status: ${res.status} ${res.statusText}`);
      return null;
    }

    dispatch({ type: "set_subscription", status: await res.json(), organizationId: currentOrganization.id });
    navigate(routerState.name, { force: "reload-orgs" });
  };

  return { subscription: currentOrganization?.current_subscription, navigateToSales, reloadSubscription };
}
