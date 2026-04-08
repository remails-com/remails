import { Button, Code, Group, Stack, Table, Title, Tooltip } from "@mantine/core";
import { modals } from "@mantine/modals";
import { ReactNode } from "react";
import OrganizationHeader from "./OrganizationHeader";
import InfoAlert from "../InfoAlert";
import { useAuditLogEntries } from "../../hooks/useAuditLog";
import StyledTable from "../StyledTable";
import { formatDateTime } from "../../util";
import TableId from "../TableId";
import { AuditLogEntry } from "../../types";
import { IconEye, IconKey, IconMail, IconServer, IconServer2, IconUser, IconUserPlus, IconWorldWww } from "@tabler/icons-react";

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
  invite_link: <IconUserPlus size={20} />,
  member: <IconUser size={20} />
};

function Actor({ entry }: { entry: AuditLogEntry }) {
  return (
    <Group gap="xs" wrap="nowrap">
      <Tooltip label={entry.actor_type.replaceAll("_", " ")}>
        {ACTOR_ICONS[entry.actor_type]}
      </Tooltip>
      {entry.actor_id ? <TableId id={entry.actor_id} /> : null}
    </Group>
  );
}

function Target({ entry }: { entry: AuditLogEntry }) {
  if (!entry.target_type || !entry.target_id) return;

  return (
    <Group gap="xs" wrap="nowrap">
      <Tooltip label={entry.target_type.replaceAll("_", " ")}>
        {TARGET_ICONS[entry.target_type]}
      </Tooltip>
      <TableId id={entry.target_id} />
    </Group>
  );
}

export default function AuditLog() {
  const { auditLogEntries } = useAuditLogEntries();

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

  const rows = auditLogEntries?.map((entry) => {
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
    )
  });

  return (
    <>
      <OrganizationHeader allowRename />
      <Title order={3} mb="md">
        Audit log
      </Title>
      <InfoAlert stateName="audit-log">
        The audit log provides a record of important actions and events that have occurred within your organization.
      </InfoAlert>

      <StyledTable headers={["Action", "Target", "Performed by", "Occurred at", ""]}>
        {rows}
      </StyledTable>
    </>
  );
}
