import { Button, Flex, Table, Title } from "@mantine/core";
import { IconUserPlus } from "@tabler/icons-react";
import NewInvite from "./NewInvite";
import { useDisclosure } from "@mantine/hooks";
import { CreatedInvite } from "../../types";
import { useState } from "react";
import { useOrganizations } from "../../hooks/useOrganizations";
import { errorNotification } from "../../notify";
import { useInvites } from "../../hooks/useInvites";
import StyledTable from "../StyledTable";
import { formatDateTime } from "../../util";
import { useSelector } from "../../hooks/useSelector";

export default function Invites() {
  const { currentOrganization } = useOrganizations();
  const { invites, setInvites } = useInvites();
  const user = useSelector((state) => state.user);

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
    invite.created_by_name = user.name;
    setInvite(invite);
    setInvites([...(invites ?? []), invite]);
    open();
  };

  const rows =
    invites?.map((invite) => (
      <Table.Tr key={invite.id}>
        <Table.Td>{formatDateTime(invite.expires_at)}</Table.Td>
        <Table.Td>{invite.created_by_name}</Table.Td>
      </Table.Tr>
    )) ?? [];

  return (
    <>
      <Title order={3} mb="md">
        Active invite links
      </Title>
      <StyledTable headers={["Expires", "Created by"]}>{rows}</StyledTable>
      <Flex justify="center" mt="md">
        <Button onClick={createInvite} leftSection={<IconUserPlus />}>
          New invite link
        </Button>
      </Flex>
      <NewInvite opened={opened} close={close} invite={invite} />
    </>
  );
}
