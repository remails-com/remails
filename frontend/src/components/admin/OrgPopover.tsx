import { ActionIcon, Anchor, Popover, Table } from "@mantine/core";
import { OrgRole } from "../../types.ts";
import TableId from "../TableId.tsx";
import { useOrganizations } from "../../hooks/useOrganizations.ts";
import { useRemails } from "../../hooks/useRemails.ts";
import { IconEye } from "@tabler/icons-react";

export default function OrgPopover({ orgs }: { orgs: OrgRole[] }) {
  const { organizations } = useOrganizations();
  const { navigate } = useRemails();

  if (orgs.length === 0) {
    return null;
  }

  return (
    <Popover>
      <Popover.Target>
        <ActionIcon size="sm" variant="outline">
          <IconEye />
        </ActionIcon>
      </Popover.Target>
      <Popover.Dropdown>
        <Table>
          <Table.Thead>
            <Table.Tr>
              <Table.Th>ID</Table.Th>
              <Table.Th>Name</Table.Th>
              <Table.Th>Role</Table.Th>
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>
            {orgs.map((org) => (
              <Table.Tr id={org.org_id}>
                <Table.Td>
                  <TableId id={org.org_id} />
                </Table.Td>
                <Table.Td>
                  <Anchor onClick={() => navigate("settings.members", { org_id: org.org_id })}>
                    {organizations.find((o) => o.id === org.org_id)?.name}
                  </Anchor>
                </Table.Td>
                <Table.Td>{org.role}</Table.Td>
              </Table.Tr>
            ))}
          </Table.Tbody>
        </Table>
      </Popover.Dropdown>
    </Popover>
  );
}
