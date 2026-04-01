import { Anchor, Badge, Button, Flex, Group, Pagination, Stack, Table } from "@mantine/core";
import { formatDateTime } from "../../util";
import { useRemails } from "../../hooks/useRemails.ts";
import { useDisclosure, useScrollIntoView } from "@mantine/hooks";
import { IconCanary, IconGavel, IconSquare, IconSquareCheck } from "@tabler/icons-react";
import StyledTable from "../StyledTable.tsx";
import { useOrganizations } from "../../hooks/useOrganizations.ts";
import TableId from "../TableId.tsx";
import { useState } from "react";
import ManageOrganization from "./ManageOrganization.tsx";
import { Organization } from "../../types.ts";

const PER_PAGE = 20;

export default function OrganizationsOverview() {
  const { currentOrganization, organizations } = useOrganizations();
  const [opened, { open, close }] = useDisclosure(false);
  const [managingOrg, setManagingOrg] = useState<Organization | null>(null);
  const [activePage, setPage] = useState(1);

  const {
    state: { config },
    navigate,
  } = useRemails();

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
      <Table.Td>
        <Group>
          {organization.name}
          {config && organization.moneybird_contact_id && (
            <Anchor
              component="a"
              href={`https://moneybird.com/${config.moneybird_administration_id}/contacts/${organization.moneybird_contact_id}`}
              target="_blank"
            >
              <Badge variant="light" color="blue" tt="none" style={{ cursor: "pointer" }} rightSection={<IconCanary size={14} />}>
                {organization.moneybird_contact_id}
              </Badge>
            </Anchor>
          )}
          {organization.block_status !== "not_blocked" && (
            <Badge variant="light" color="red">
              {organization.block_status.replaceAll("_", " ")}
            </Badge>
          )}
        </Group>
      </Table.Td>
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
      <Table.Td align="right">
        {organization.used_message_quota} / {organization.total_message_quota}
      </Table.Td>
      <Table.Td w={150} visibleFrom="lg">{formatDateTime(organization.updated_at)}</Table.Td>
      <Table.Td w={150} visibleFrom="xl">{formatDateTime(organization.created_at)}</Table.Td>
      <Table.Td align="right" pl="0">
        <Button variant="subtle" onClick={() => {
          setManagingOrg(organization);
          open();
        }}>
          <IconGavel />
        </Button>
      </Table.Td>
    </Table.Tr>
  ));

  return (
    <>
      <ManageOrganization opened={opened} close={close} organization={managingOrg} key={managingOrg?.id} />
      <StyledTable ref={targetRef} headers={[
        "ID", "Name", "",
        { ta: "right", children: "Quota" },
        { visibleFrom: "lg", children: "Updated" },
        { visibleFrom: "xl", children: "Created" },
        ""
      ]}>
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
        </Stack>
      </Flex>
    </>
  );
}
