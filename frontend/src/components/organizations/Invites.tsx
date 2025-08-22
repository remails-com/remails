import { ActionIcon, Button, Flex, Table, Text } from "@mantine/core";
import { IconTrash, IconUserPlus } from "@tabler/icons-react";
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
import { modals } from "@mantine/modals";
import { notifications } from "@mantine/notifications";

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

  const deleteInvite = async (id: string) => {
    const res = await fetch(`/api/invite/${currentOrganization.id}/${id}`, {
      method: "DELETE",
    });
    if (res.status === 200) {
      setInvites((invites) => invites?.filter((invite) => invite.id !== id) ?? []);
      notifications.show({
        title: "Invite deleted",
        message: "Invite was deleted",
        color: "green",
      });
    } else {
      errorNotification("Invite could not be deleted");
      console.error(res);
    }
  };

  const confirmDeleteInvite = (id: string) => {
    modals.openConfirmModal({
      title: "Please confirm your action",
      children: <Text>Are you sure you want to delete this invite?</Text>,
      labels: { confirm: "Confirm", cancel: "Cancel" },
      onCancel: () => {},
      onConfirm: () => deleteInvite(id),
    });
  };

  const rows = invites?.map((invite) => (
    <Table.Tr key={invite.id}>
      <Table.Td>{formatDateTime(invite.expires_at)}</Table.Td>
      <Table.Td>{invite.created_by_name}</Table.Td>
      <Table.Td align={"right"}>
        <ActionIcon variant="light" onClick={() => confirmDeleteInvite(invite.id)} size={30}>
          <IconTrash />
        </ActionIcon>
      </Table.Td>
    </Table.Tr>
  ));

  return (
    <>
      <StyledTable headers={["Expires", "Created by", ""]}>{rows}</StyledTable>
      <Flex justify="center" mt="md">
        <Button onClick={createInvite} leftSection={<IconUserPlus />}>
          New invite link
        </Button>
      </Flex>
      <NewInvite opened={opened} close={close} invite={invite} />
    </>
  );
}
