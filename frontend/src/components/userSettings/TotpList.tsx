import { ActionIcon, Table, Text } from "@mantine/core";
import { formatDateTime } from "../../util.ts";
import StyledTable from "../StyledTable.tsx";
import { IconTrash, IconX } from "@tabler/icons-react";
import useTotpCodes from "../../hooks/useTotpCodes.ts";
import { useRemails } from "../../hooks/useRemails.ts";
import { notifications } from "@mantine/notifications";

export default function TotpList() {
  const { totpCodes } = useTotpCodes();
  const {
    dispatch,
    state: { user },
  } = useRemails();

  if (totpCodes === null || user === null) {
    return null;
  }

  const formatLastUsed = (lastUsed: string | null) => {
    if (lastUsed === null) {
      return (
        <Text c="dimmed" fs="italic">
          Never
        </Text>
      );
    }
    return formatDateTime(lastUsed);
  };

  const formatDescription = (description: string) => {
    if (description === "") {
      return (
        <Text c="dimmed" fs="italic">
          No description
        </Text>
      );
    }
    return description;
  };

  const deleteCode = async (id: string) => {
    const res = await fetch(`/api/api_user/${user.id}/totp/${id}`, {
      method: "DELETE",
    });
    if (res.status === 200) {
      notifications.show({
        title: "2FA method deleted",
        message: "",
        color: "green",
      });
      dispatch({ type: "remove_totp_code", totpCodeId: id });
    } else {
      notifications.show({
        title: "Error",
        message: "2FA method deleted could not be deleted",
        color: "red",
        autoClose: 20000,
        icon: <IconX size={20} />,
      });
      console.error(res);
    }
  };

  return (
    <StyledTable headers={["Description", "Last Used", ""]}>
      {totpCodes.map((code) => (
        <Table.Tr key={code.id}>
          <Table.Td>{formatDescription(code.description)}</Table.Td>
          <Table.Td>{formatLastUsed(code.last_used)}</Table.Td>
          <Table.Td>
            <ActionIcon variant="outline">
              <IconTrash onClick={() => deleteCode(code.id)} />
            </ActionIcon>
          </Table.Td>
        </Table.Tr>
      ))}
    </StyledTable>
  );
}
