import { useApiUsers } from "../../hooks/useApiUsers.ts";
import StyledTable from "../StyledTable.tsx";
import { Anchor, Flex, Group, Pagination, Table } from "@mantine/core";
import TableId from "../TableId.tsx";
import RoleSelect from "./RoleSelect.tsx";
import OrgPopover from "./OrgPopover.tsx";
import { formatDateTime } from "../../util.ts";
import { useState } from "react";
import { useScrollIntoView } from "@mantine/hooks";

const PER_PAGE = 20;

export default function ApiUserOverview() {
  const [activePage, setPage] = useState(1);
  const { apiUsers } = useApiUsers();

  const { scrollIntoView, targetRef } = useScrollIntoView<HTMLDivElement>({
    duration: 500,
    offset: 100,
  });

  const rows = apiUsers.slice((activePage - 1) * PER_PAGE, activePage * PER_PAGE).map((user) => (
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
          {user.org_roles.length.toString()}
          <OrgPopover orgs={user.org_roles} />
        </Group>
      </Table.Td>
      <Table.Td w={150}>{formatDateTime(user.created_at)}</Table.Td>
      <Table.Td w={150}>{formatDateTime(user.updated_at)}</Table.Td>
    </Table.Tr>
  ));

  return (
    <div ref={targetRef}>
      <StyledTable headers={["ID", "Name", "Email", "Global Role", "Organizations", "Created", "Updated"]}>
        {rows}
      </StyledTable>
      <Flex justify="center" mt="md">
        <Pagination
          value={activePage}
          onChange={(p) => {
            setPage(p);
            scrollIntoView({ alignment: "start" });
          }}
          total={Math.ceil(apiUsers.length / PER_PAGE)}
        />
      </Flex>
    </div>
  );
}
