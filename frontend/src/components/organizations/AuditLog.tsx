import { Button, Code, Flex, Group, Pagination, Stack, Table, Text, Title, Tooltip } from "@mantine/core";
import { modals } from "@mantine/modals";
import { ReactNode, useState } from "react";
import OrganizationHeader from "./OrganizationHeader";
import InfoAlert from "../InfoAlert";
import { useAuditLogEntries } from "../../hooks/useAuditLog";
import StyledTable from "../StyledTable";
import { formatDateTime } from "../../util";
import TableId from "../TableId";
import { AuditLogEntry } from "../../types";
import { IconEye, IconKey, IconLinkPlus, IconMail, IconServer, IconServer2, IconUser, IconWorldWww } from "@tabler/icons-react";
import { Loader } from "../../Loader";
import { useRemails } from "../../hooks/useRemails";
import SearchInput from "../SearchInput";
import { useMemberWithId, useMembers } from "../../hooks/useOrganizations";
import { useProjectWithId, useProjects } from "../../hooks/useProjects";
import { useDomainWithId, useDomains } from "../../hooks/useDomains";

const PER_PAGE = 20;
const SHOW_SEARCH = 10;

const ACTOR_ICONS: Record<AuditLogEntry["actor_type"], ReactNode> = {
  api_key: <IconKey size={20} />,
  api_user: <IconUser size={20} />,
  system: <IconServer2 size={20} />,
};

const TARGET_ICONS: Record<NonNullable<AuditLogEntry["target_type"]>, ReactNode> = {
  api_key: <IconKey size={20} />,
  domain: <IconWorldWww size={20} />,
  message: <IconMail size={20} />,
  project: <IconServer size={20} />,
  smtp_credential: <IconKey size={20} />,
  invite_link: <IconLinkPlus size={20} />,
  member: <IconUser size={20} />
};

function Actor({ entry }: { entry: AuditLogEntry }) {
  const member = useMemberWithId(entry.actor_type === "api_user" ? entry.actor_id : null);

  return (
    <Group gap="xs" wrap="nowrap">
      <Tooltip label={entry.actor_type.replaceAll("_", " ")}>
        {ACTOR_ICONS[entry.actor_type]}
      </Tooltip>
      {(entry.actor_id ? <TableId id={entry.actor_id} name={member?.name} /> : "System")}
    </Group>
  );
}

function Target({ entry }: { entry: AuditLogEntry }) {
  const names: Record<NonNullable<AuditLogEntry["target_type"]>, string | undefined> = {
    member: useMemberWithId(entry.target_type === "member" ? entry.target_id : null)?.name,
    project: useProjectWithId(entry.target_type === "project" ? entry.target_id : null)?.name,
    api_key: undefined,
    domain: useDomainWithId(entry.target_type === "domain" ? entry.target_id : null)?.domain,
    message: undefined,
    smtp_credential: undefined,
    invite_link: undefined
  }

  if (!entry.target_type || !entry.target_id) return;

  return (
    <Group gap="xs" wrap="nowrap">
      <Tooltip label={entry.target_type.replaceAll("_", " ")}>
        {TARGET_ICONS[entry.target_type]}
      </Tooltip>
      <TableId id={entry.target_id} name={names[entry.target_type]} />
    </Group>
  );
}

function AuditLogTable() {
  const {
    state: { routerState },
    navigate,
  } = useRemails();
  const { auditLogEntries } = useAuditLogEntries();
  const [searchQuery, setSearchQuery] = useState(routerState.params.q || "");
  const { members } = useMembers();
  const { projects } = useProjects();
  const { domains } = useDomains();

  const openDetailsModal = (entry: AuditLogEntry) => {
    modals.open({
      title: <Title order={3} component="span">{entry.action}</Title>,
      size: "lg",
      children: (
        <Stack>
          <Table variant="vertical" withTableBorder>
            <Table.Tbody>
              <Table.Tr>
                <Table.Th w="128">Target</Table.Th>
                <Table.Td><Target entry={entry} /></Table.Td>
              </Table.Tr>
              <Table.Tr>
                <Table.Th w="128">Performed by</Table.Th>
                <Table.Td><Actor entry={entry} /></Table.Td>
              </Table.Tr>
              <Table.Tr>
                <Table.Th w="128">Occurred at</Table.Th>
                <Table.Td>{formatDateTime(entry.occurred_at)}</Table.Td>
              </Table.Tr>
            </Table.Tbody>
          </Table>

          <Stack gap={0}>
            Additional event details:
            <Code block>
              {JSON.stringify(entry.details, null, 4)}
            </Code>
          </Stack>
        </Stack>
      ),
    });
  };

  if (auditLogEntries === null) {
    return <Loader />;
  }

  const normalizedSearchQuery = searchQuery.toLowerCase();
  const filteredEntries =
    searchQuery.length == 0
      ? auditLogEntries
      : auditLogEntries.filter((entry) =>
        [
          entry.action,
          entry.actor_type,
          entry.actor_id ?? "",
          members.find((member) => member.user_id === entry.actor_id)?.name ?? "",
          entry.target_type ?? "",
          entry.target_id ?? "",
          projects.find((project) => project.id === entry.target_id)?.name ?? "",
          domains.find((domain) => domain.id === entry.target_id)?.domain ?? "",
          JSON.stringify(entry.details),
        ].some((value) => value.toLowerCase().includes(normalizedSearchQuery))
      );

  const totalPages = Math.ceil(filteredEntries.length / PER_PAGE);
  const activePage = Math.min(Math.max(parseInt(routerState.params.p) || 1, 1), totalPages || 1);

  const rows = filteredEntries.slice((activePage - 1) * PER_PAGE, activePage * PER_PAGE).map((entry) => {
    return (
      <Table.Tr key={entry.id}>
        <Table.Td>
          {entry.action}
        </Table.Td>
        <Table.Td><Target entry={entry} /></Table.Td>
        <Table.Td><Actor entry={entry} /></Table.Td>
        <Table.Td>{formatDateTime(entry.occurred_at)}</Table.Td>
        <Table.Td align="right" pl="0">
          <Tooltip label="View event details">
            <Button variant="subtle" size="xs" onClick={() => openDetailsModal(entry)}>
              <IconEye />
            </Button>
          </Tooltip>
        </Table.Td>
      </Table.Tr>
    );
  });

  return (
    <>
      {(auditLogEntries.length > SHOW_SEARCH || searchQuery.length > 0) && (
        <SearchInput searchQuery={searchQuery} setSearchQuery={setSearchQuery} />
      )}

      {searchQuery.length > 0 && filteredEntries.length == 0 && (
        <Text fs="italic" c="gray">
          No audit log entries found...
        </Text>
      )}

      <StyledTable headers={["Action", "Target", "Performed by", "Occurred at", ""]}>
        {rows}
      </StyledTable>

      <Flex justify="center" mt="md">
        {filteredEntries.length > PER_PAGE && (
          <Pagination
            value={activePage}
            onChange={(p) => {
              navigate(routerState.name, {
                ...routerState.params,
                p: p.toString(),
              });
            }}
            total={totalPages}
          />
        )}
      </Flex>
    </>
  );
}

export default function AuditLog() {
  return (
    <>
      <OrganizationHeader allowRename />
      <Title order={3} mb="md">
        Audit log
      </Title>
      <InfoAlert stateName="audit-log">
        The audit log provides a record of important actions and events that have occurred within your organization.
      </InfoAlert>
      <AuditLogTable />
    </>
  );
}
