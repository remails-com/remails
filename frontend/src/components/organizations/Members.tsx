import { ActionIcon, Alert, Button, Flex, Table, Text, Title, Tooltip } from "@mantine/core";
import { IconInfoCircle, IconTrash, IconUserMinus, IconUserPlus } from "@tabler/icons-react";
import NewInvite from "./NewInvite";
import { useDisclosure } from "@mantine/hooks";
import { CreatedInvite } from "../../types";
import { useState } from "react";
import { useInvites, useMembers, useOrganizations } from "../../hooks/useOrganizations";
import { errorNotification } from "../../notify";
import StyledTable from "../StyledTable";
import { formatDateTime } from "../../util";
import { useSelector } from "../../hooks/useSelector";
import { modals } from "@mantine/modals";
import { notifications } from "@mantine/notifications";
import InfoAlert from "../InfoAlert";
import { useRemails } from "../../hooks/useRemails";

export default function Members() {
  const { currentOrganization } = useOrganizations();
  const { invites, setInvites } = useInvites();
  const { members } = useMembers();
  const user = useSelector((state) => state.user);
  const { navigate } = useRemails();

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

  const invite_rows = invites?.map((invite) => (
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

  const removeFromOrganization = async (id: string) => {
    const res = await fetch(`/api/organizations/${currentOrganization.id}/members/${id}`, {
      method: "DELETE",
    });
    if (res.status === 200) {
      setInvites((invites) => invites?.filter((invite) => invite.id !== id) ?? []);
      notifications.show({
        title: "User removed",
        message: "User removed from organization",
        color: "green",
      });
      if (id == user.id) {
        // reload-orgs makes sure that the organization is removed from the front-end
        navigate("default", { force: "reload-orgs" });
      }
    } else {
      errorNotification("Could not remove user from organization");
      console.error(res);
    }
  };

  const confirmLeaveOrg = () => {
    modals.openConfirmModal({
      title: "Please confirm your action",
      children: (
        <>
          <Text>
            Are you sure you want to leave the{" "}
            <Text fw="bold" span>
              {currentOrganization.name}
            </Text>{" "}
            organization?
          </Text>
          <Alert my="lg" icon={<IconInfoCircle />}>
            You will not be able to access this organization anymore, unless you get reinvited by one of the remaining
            admins.
          </Alert>
        </>
      ),
      labels: { confirm: "Confirm", cancel: "Cancel" },
      onCancel: () => {},
      onConfirm: () => removeFromOrganization(user.id),
    });
  };

  const is_last_remaining_admin =
    user.org_roles.some((o) => o.org_id == currentOrganization.id && o.role == "admin") &&
    !members?.some((m) => m.role == "admin" && m.user_id != user.id);

  const member_rows = members?.map((member) => (
    <Table.Tr key={member.user_id}>
      <Table.Td>{member.name}</Table.Td>
      <Table.Td>{member.email}</Table.Td>
      <Table.Td>{member.role}</Table.Td>
      <Table.Td>{formatDateTime(member.updated_at)}</Table.Td>
      <Table.Td align={"right"} h="51">
        {member.user_id == user.id && (
          <Tooltip
            label={
              is_last_remaining_admin
                ? "The last remaining admin in this organization cannot leave. Perhaps you want to delete this organization instead?"
                : "Leave organization"
            }
          >
            <Button onClick={confirmLeaveOrg} disabled={is_last_remaining_admin} leftSection={<IconUserMinus />}>
              Leave
            </Button>
          </Tooltip>
        )}
      </Table.Td>
    </Table.Tr>
  ));

  return (
    <>
      <InfoAlert stateName="organization-members">
        This section shows all Remails accounts that have access to this organization. Admins can invite new members to
        this organization by creating and sharing invite links.
      </InfoAlert>

      <Title order={3} mb="md">
        Organization members
      </Title>
      <StyledTable headers={["Name", "Email", "Role", "Updated", ""]}>{member_rows}</StyledTable>

      <Title order={3} mb="md" mt="xl">
        Organization invites
      </Title>
      <StyledTable headers={["Expires", "Created by", ""]}>{invite_rows}</StyledTable>
      <Flex justify="center" mt="md">
        <Button onClick={createInvite} leftSection={<IconUserPlus />}>
          New invite link
        </Button>
      </Flex>
      <NewInvite opened={opened} close={close} invite={invite} />
    </>
  );
}
