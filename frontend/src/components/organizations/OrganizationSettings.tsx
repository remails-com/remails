import { Container, Stack, Title } from "@mantine/core";
import SubscriptionCard from "./SubscriptionCard.tsx";
import { useOrganizations } from "../../hooks/useOrganizations.ts";
import Rename from "../Rename.tsx";
import { notifications } from "@mantine/notifications";
import { IconX } from "@tabler/icons-react";
import { useRemails } from "../../hooks/useRemails.ts";

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
      <Stack>
        <Rename name={currentOrganization.name} save={save}></Rename>
        <Title order={3}>Your subscription</Title>
        <SubscriptionCard />
      </Stack>
    </Container>
  );
}
