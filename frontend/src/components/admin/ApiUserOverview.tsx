import { useApiUsers } from "../../hooks/useApiUsers.ts";
import StyledTable from "../StyledTable.tsx";
import { Anchor, Group, Table } from "@mantine/core";
import TableId from "../TableId.tsx";
import RoleSelect from "./RoleSelect.tsx";
import OrgPopover from "./OrgPopover.tsx";
import { formatDateTime } from "../../util.ts";

export default function ApiUserOverview() {
  const { apiUsers } = useApiUsers();

  const rows = apiUsers.map((user) => (
    <Table.Tr key={user.id}>
      <Table.Td w={80}>
        <TableId id={user.id} />
      </Table.Td>
      <Table.Td>{user.name}</Table.Td>
      <Table.Td>
        <Anchor component="a" href={`mailto:${user.email}`}>
          {user.email}
        </Anchor>
      </Table.Td>
      <Table.Td w={120}>
        <RoleSelect id={user.id} role={user.global_role} />
      </Table.Td>
      <Table.Td align="center" w={120}>
        <Group justify="center">
          {user.org_roles.length}
          <OrgPopover orgs={user.org_roles} />
        </Group>
      </Table.Td>
      <Table.Td w={150}>{formatDateTime(user.created_at)}</Table.Td>
      <Table.Td w={150}>{formatDateTime(user.updated_at)}</Table.Td>
    </Table.Tr>
  ));

  return (
    <>
      <StyledTable headers={["ID", "Name", "Email", "Global Role", "Organizations", "Created", "Updated"]}>
        {rows}
      </StyledTable>
    </>
  );
}
