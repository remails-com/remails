import { Table } from "@mantine/core";
import { Loader } from "../../Loader";
import { useOrganizations } from "../../hooks/useOrganizations";
import { formatDateTime } from "../../util";

export function OrganizationsOverview() {
  const { organizations, loading } = useOrganizations();

  if (loading) {
    return <Loader />;
  }

  const rows = organizations.map((organization) => (
    <Table.Tr key={organization.id}>
      <Table.Td>{organization.name}</Table.Td>
      <Table.Td>{formatDateTime(organization.updated_at)}</Table.Td>
    </Table.Tr>
  ));

  return (
    <Table>
      <Table.Thead>
        <Table.Tr>
          <Table.Th>Name</Table.Th>
          <Table.Th>Updated</Table.Th>
        </Table.Tr>
      </Table.Thead>
      <Table.Tbody>{rows}</Table.Tbody>
    </Table>
  );
}