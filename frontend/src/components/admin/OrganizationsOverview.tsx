import { ActionIcon, Anchor, Button, Flex, Pagination, Stack, Table } from "@mantine/core";
import { formatDateTime } from "../../util";
import { useRemails } from "../../hooks/useRemails.ts";
import { useDisclosure, useScrollIntoView } from "@mantine/hooks";
import { IconExternalLink, IconGavel, IconPlus, IconSquare, IconSquareCheck } from "@tabler/icons-react";
import StyledTable from "../StyledTable.tsx";
import { useOrganizations } from "../../hooks/useOrganizations.ts";
import { NewOrganization } from "../organizations/NewOrganization.tsx";
import TableId from "../TableId.tsx";
import { useState } from "react";

const PER_PAGE = 20;

export default function OrganizationsOverview() {
  const [opened, { open, close }] = useDisclosure(false);
  const { currentOrganization, organizations } = useOrganizations();
  const [activePage, setPage] = useState(1);

  const {
    state: { config },
  } = useRemails();
  const { navigate } = useRemails();

  const { scrollIntoView, targetRef } = useScrollIntoView<HTMLTableSectionElement>({
    duration: 500,
    offset: 100,
  });

  const rows = organizations.slice((activePage - 1) * PER_PAGE, activePage * PER_PAGE).map((organization) => (
    <Table.Tr
      key={organization.id}
      bg={currentOrganization?.id == organization.id ? "var(--mantine-color-blue-light)" : undefined}
    >
      <Table.Td w={80}>
        <TableId id={organization.id} />
      </Table.Td>
      <Table.Td>{organization.name}</Table.Td>
      <Table.Td>
        <Button
          leftSection={currentOrganization?.id == organization.id ? <IconSquareCheck /> : <IconSquare />}
          variant="subtle"
          onClick={() => {
            navigate("admin.organizations", { org_id: organization.id });
          }}
        >
          Act as this organization
        </Button>
      </Table.Td>
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
      <Table.Td>
        {organization.used_message_quota} / {organization.total_message_quota}
      </Table.Td>
      <Table.Td w={150}>{formatDateTime(organization.updated_at)}</Table.Td>
      <Table.Td align={"right"} pl="0">
        <ActionIcon
          size="30"
          variant="subtle"
          onClick={() => {
            navigate("admin", { org_id: organization.id });
          }}
        >
          <IconGavel />
        </ActionIcon>
      </Table.Td>
    </Table.Tr>
  ));

  return (
    <>
      <NewOrganization
        opened={opened}
        close={close}
        done={(newOrg) => navigate("admin.organizations", { org_id: newOrg.id })}
      />
      <StyledTable ref={targetRef} headers={["ID", "Name", "", "Moneybird contact ID", "Quota", "Updated", ""]}>
        {rows}
      </StyledTable>

      <Flex justify="center" mt="md">
        <Stack>
          <Pagination
            value={activePage}
            onChange={(p) => {
              setPage(p);
              scrollIntoView({ alignment: "start" });
            }}
            total={Math.ceil(organizations.length / PER_PAGE)}
          />
          <Button onClick={() => open()} leftSection={<IconPlus />}>
            New Organization
          </Button>
        </Stack>
      </Flex>
    </>
  );
}
