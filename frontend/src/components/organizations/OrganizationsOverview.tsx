import { Button, Flex, Table, Text, Tooltip } from "@mantine/core";
import { Loader } from "../../Loader";
import { formatDateTime } from "../../util";
import { useOrganizations } from "../../hooks/useOrganizations.ts";
import { useRemails } from "../../hooks/useRemails.ts";
import { useDisclosure } from "@mantine/hooks";
import { IconPlus, IconSquare, IconSquareCheck } from "@tabler/icons-react";
import { NewOrganization } from "./NewOrganization.tsx";

export default function OrganizationsOverview() {
  const [opened, { open, close }] = useDisclosure(false);
  const { organizations, currentOrganization } = useOrganizations();
  const { navigate } = useRemails();

  if (!organizations) {
    return <Loader />;
  }

  const rows = organizations.map((organization) => (
    <Table.Tr
      key={organization.id}
      bg={currentOrganization?.id == organization.id ? "var(--mantine-color-blue-light)" : undefined}
    >
      <Table.Td>
        <Tooltip label={organization.id}>
          <Text span c={"dimmed"} size="sm">
            {organization.id.substring(0, 8)}
          </Text>
        </Tooltip>
      </Table.Td>
      <Table.Td>{organization.name}</Table.Td>
      <Table.Td>{formatDateTime(organization.updated_at)}</Table.Td>
      <Table.Td align={"right"}>
        <Button
          rightSection={currentOrganization?.id == organization.id ? <IconSquareCheck /> : <IconSquare />}
          variant="subtle"
          onClick={() => {
            navigate("organizations", { org_id: organization.id });
          }}
        >
          Act as this organization
        </Button>
      </Table.Td>
    </Table.Tr>
  ));

  return (
    <>
      <NewOrganization
        opened={opened}
        close={close}
        done={(newOrg) => navigate("organizations", { org_id: newOrg.id })}
      />
      <Table highlightOnHover>
        <Table.Thead>
          <Table.Tr>
            <Table.Th>ID</Table.Th>
            <Table.Th>Name</Table.Th>
            <Table.Th>Updated</Table.Th>
            <Table.Th></Table.Th>
          </Table.Tr>
        </Table.Thead>
        <Table.Tbody>{rows}</Table.Tbody>
      </Table>
      <Flex justify="center" mt="md">
        <Button onClick={() => open()} leftSection={<IconPlus />}>
          New Organization
        </Button>
      </Flex>
    </>
  );
}
