import { Button, Title } from "@mantine/core";
import { IconUserPlus } from "@tabler/icons-react";
import NewInvite from "./NewInvite";
import { useDisclosure } from "@mantine/hooks";
import { CreatedInvite } from "../../types";
import { useState } from "react";
import { useOrganizations } from "../../hooks/useOrganizations";
import { errorNotification } from "../../notify";

export default function Invites() {
  const { currentOrganization } = useOrganizations();

  const [opened, { open, close }] = useDisclosure(false);
  const [invite, setInvite] = useState<CreatedInvite | null>(null);

  if (!currentOrganization) {
    return null;
  }

  const createInvite = async () => {
    const res = await fetch(`/api/invite/${currentOrganization.id}`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
    });

    if (res.status !== 201) {
      errorNotification("Could not create invite");
      console.error(res);
      return;
    }

    const invite = await res.json();
    setInvite(invite);
    open();
  };

  return (
    <>
      <Title order={3} mb="md">
        Organization invites
      </Title>
      <Button leftSection={<IconUserPlus />} onClick={createInvite}>
        Create invite link
      </Button>
      <NewInvite opened={opened} close={close} invite={invite} />
    </>
  );
}
