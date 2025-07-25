import { Container, Title } from "@mantine/core";
import SubscriptionCard from "./SubscriptionCard.tsx";
import { useOrganizations } from "../../hooks/useOrganizations.ts";
import { notifications } from "@mantine/notifications";
import { IconBuildings, IconX } from "@tabler/icons-react";
import { useRemails } from "../../hooks/useRemails.ts";
import EntityHeader from "../EntityHeader.tsx";

export default function OrganizationSettings() {
  const { currentOrganization } = useOrganizations();
  const { dispatch } = useRemails();

  if (!currentOrganization) {
    return null;
  }

  const save = async (values: { name: string }) => {
    const res = await fetch(`/api/organizations/${currentOrganization.id}`, {
      method: "PUT",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(values),
    });
    if (res.status !== 200) {
      notifications.show({
        title: "Error",
        message: `Organization could not be updated`,
        color: "red",
        autoClose: 20000,
        icon: <IconX size={20} />,
      });
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
    <Container size="xs" mt="md" pl="0" ml="0">
      <EntityHeader name={currentOrganization.name} entityType="Organization" Icon={IconBuildings} saveRename={save} />
      <Title order={3} mb="md">
        Your subscription
      </Title>
      <SubscriptionCard />
    </Container>
  );
}
