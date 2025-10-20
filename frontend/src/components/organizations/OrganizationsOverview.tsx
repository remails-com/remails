import { Anchor, Button, Flex, Table, Text, Tooltip } from "@mantine/core";
import { formatDateTime } from "../../util";
import { useRemails } from "../../hooks/useRemails.ts";
import { useDisclosure } from "@mantine/hooks";
import { IconExternalLink, IconPlus, IconSquare, IconSquareCheck } from "@tabler/icons-react";
import { NewOrganization } from "./NewOrganization.tsx";
import StyledTable from "../StyledTable.tsx";
import { useOrganizations } from "../../hooks/useOrganizations.ts";

export default function OrganizationsOverview() {
  const [opened, { open, close }] = useDisclosure(false);
  const { currentOrganization, organizations } = useOrganizations();
  const {
    state: { config },
  } = useRemails();
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
      <Table.Td>
        {config && organization.moneybird_contact_id && (
          <Anchor
            component="a"
            href={`https://moneybird.com/${config.moneybird_administration_id}/contacts/${organization.moneybird_contact_id}`}
            target="_blank"
          >
            {organization.moneybird_contact_id}
            <IconExternalLink size={18} />
          </Anchor>
        )}
      </Table.Td>
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
      <StyledTable headers={["ID", "Name", "Updated", "Moneybird contact ID", ""]}>{rows}</StyledTable>

      <Flex justify="center" mt="md">
        <Button onClick={() => open()} leftSection={<IconPlus />}>
          New Organization
        </Button>
      </Flex>
    </>
  );
}
