import { useApiUsers } from "../../hooks/useApiUsers.ts";
import StyledTable from "../StyledTable.tsx";
import { Badge, Button, Flex, Group, Pagination, Table } from "@mantine/core";
import TableId from "../TableId.tsx";
import OrgPopover from "./OrgPopover.tsx";
import { formatDateTime } from "../../util.ts";
import { useState } from "react";
import { useDisclosure, useScrollIntoView } from "@mantine/hooks";
import { IconEdit } from "@tabler/icons-react";
import ManageApiUser from "./ManageApiUser.tsx";
import { User } from "../../types.ts";

const PER_PAGE = 20;

export default function ApiUserOverview() {
  const [opened, { open, close }] = useDisclosure(false);
  const [managingUser, setManagingUser] = useState<User | null>(null);
  const [activePage, setPage] = useState(1);
  const { apiUsers } = useApiUsers();

  const { scrollIntoView, targetRef } = useScrollIntoView<HTMLTableSectionElement>({
    duration: 500,
    offset: 100,
  });

  const rows = apiUsers.slice((activePage - 1) * PER_PAGE, activePage * PER_PAGE).map((user) => (
    <Table.Tr key={user.id}>
      <Table.Td w={80}>
        <TableId id={user.id} />
      </Table.Td>
      <Table.Td>
        <Group>
          {user.name}
          <Badge variant="light" color="secondary" tt="none" component="a" href={`mailto:${user.email}`} style={{ cursor: "pointer" }}>
            {user.email}
          </Badge>
          {user.global_role == "admin" && (
            <Badge variant="light">
              Admin
            </Badge>
          )}
          {user.blocked && (
            <Badge variant="light" color="red">
              Blocked
            </Badge>
          )}
        </Group>
      </Table.Td>
      <Table.Td w={80}>
        <Group justify="left">
          {user.org_roles.length.toString()}
          <OrgPopover orgs={user.org_roles} />
        </Group>
      </Table.Td>
      <Table.Td w={150} visibleFrom="md">{formatDateTime(user.updated_at)}</Table.Td>
      <Table.Td w={150} visibleFrom="lg">{formatDateTime(user.created_at)}</Table.Td>
      <Table.Td align="right" pl="0">
        <Button variant="subtle" onClick={() => {
          setManagingUser(user);
          open();
        }}>
          <IconEdit />
        </Button>
      </Table.Td>
    </Table.Tr >
  ));

  return (
    <>
      <ManageApiUser opened={opened} close={close} user={managingUser} key={managingUser?.id} />
      <StyledTable ref={targetRef} headers={[
        "ID", "Name", "Orgs",
        { visibleFrom: "md", children: "Updated" },
        { visibleFrom: "lg", children: "Created" },
        ""
      ]}>
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
    </>
  );
}
