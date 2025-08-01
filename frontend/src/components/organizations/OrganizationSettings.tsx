import { Button, Container, Title } from "@mantine/core";
import SubscriptionCard from "./SubscriptionCard.tsx";
import { useOrganizations } from "../../hooks/useOrganizations.ts";
import { notifications } from "@mantine/notifications";
import { IconBuildings, IconUserPlus, IconX } from "@tabler/icons-react";
import { useRemails } from "../../hooks/useRemails.ts";
import Header from "../Header.tsx";
import { useState } from "react";
import { CreatedInvite } from "../../types.ts";
import NewInvite from "./NewInvite.tsx";
import { useDisclosure } from "@mantine/hooks";

export default function OrganizationSettings() {
  const { currentOrganization } = useOrganizations();
  const { dispatch } = useRemails();

  const [opened, { open, close }] = useDisclosure(false);
  const [invite, setInvite] = useState<CreatedInvite | null>(null);

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
      notifications.show({
        title: "Error",
        message: "Organization could not be updated",
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

  const createInvite = async () => {
    const res = await fetch(`/api/invite/${currentOrganization.id}`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
    });

    if (res.status !== 201) {
      notifications.show({
        title: "Error",
        message: "Could not create invite",
        color: "red",
        autoClose: 20000,
        icon: <IconX size={20} />,
      });
      console.error(res);
      return;
    }

    const invite = await res.json();
    setInvite(invite);
    open();
  };

  return (
    <>
      <Header
        name={currentOrganization.name}
        entityType="Organization"
        Icon={IconBuildings}
        saveRename={saveName}
        divider
      />
      <Container size="xs" mt="md" pl="0" ml="0">
        <Title order={3} mb="md">
          Users
        </Title>
        <Button leftSection={<IconUserPlus />} onClick={createInvite}>
          Create invite link
        </Button>
        <NewInvite opened={opened} close={close} invite={invite} />
        <Title order={3} my="md">
          Your subscription
        </Title>
        <SubscriptionCard />
      </Container>
    </>
  );
}
