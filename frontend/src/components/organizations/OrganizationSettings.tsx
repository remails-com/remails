import SubscriptionCard from "./SubscriptionCard.tsx";
import { useOrganizations, useOrgRole } from "../../hooks/useOrganizations.ts";
import { notifications } from "@mantine/notifications";
import { IconBuildings, IconReceiptEuro, IconUsers } from "@tabler/icons-react";
import { useRemails } from "../../hooks/useRemails.ts";
import Header from "../Header.tsx";
import { errorNotification } from "../../notify.tsx";
import Tabs from "../../layout/Tabs.tsx";
import Members from "./Members.tsx";

export default function OrganizationSettings() {
  const { currentOrganization } = useOrganizations();
  const { isAdmin } = useOrgRole();
  const { dispatch } = useRemails();

  if (!currentOrganization) {
    return null;
  }

  const saveName = async (values: { name: string }) => {
    const res = await fetch(`/api/organizations/${currentOrganization.id}`, {
      method: "PUT",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(values),
    });
    if (res.status !== 200) {
      errorNotification("Organization could not be updated");
      console.error(res);
      return;
    }
    const organization = await res.json();

    notifications.show({
      title: "Organization updated",
      message: "",
      color: "green",
    });
    dispatch({ type: "remove_organization", organizationId: currentOrganization.id });
    dispatch({ type: "add_organization", organization: organization });
  };

  return (
    <>
      <Header
        name={currentOrganization.name}
        entityType="Organization"
        Icon={IconBuildings}
        saveRename={isAdmin ? saveName : undefined}
      />
      <Tabs
        tabs={[
          {
            route: "settings",
            name: "Subscription",
            icon: <IconReceiptEuro size={14} />,
            content: <SubscriptionCard />,
          },
          {
            route: "settings.members",
            name: "Members",
            icon: <IconUsers size={14} />,
            content: <Members />,
          },
        ]}
      />
    </>
  );
}
