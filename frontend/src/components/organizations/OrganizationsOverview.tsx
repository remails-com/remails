import { Button, Flex, Table, Text, Tooltip } from "@mantine/core";
import { formatDateTime } from "../../util";
import { useRemails } from "../../hooks/useRemails.ts";
import { useDisclosure } from "@mantine/hooks";
import { IconPlus, IconSquare, IconSquareCheck } from "@tabler/icons-react";
import { NewOrganization } from "./NewOrganization.tsx";
import StyledTable from "../StyledTable.tsx";
import useSelector from "../../hooks/useSelector.ts";

export default function OrganizationsOverview() {
  const [opened, { open, close }] = useDisclosure(false);
  const organizations = useSelector((state) => state.organizations);
  const currentOrganization = useSelector((state) =>
    state.organizations?.find((o) => o.id === state.routerState.params.org_id)
  );
  const { navigate } = useRemails();

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
      <StyledTable headers={["ID", "Name", "Updated", ""]}>{rows}</StyledTable>

      <Flex justify="center" mt="md">
        <Button onClick={() => open()} leftSection={<IconPlus />}>
          New Organization
        </Button>
      </Flex>
    </>
  );
}
