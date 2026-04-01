import { Table, Title } from "@mantine/core";
import OrganizationHeader from "./OrganizationHeader";
import InfoAlert from "../InfoAlert";
import { useAuditLogEntries } from "../../hooks/useOrganizations";
import StyledTable from "../StyledTable";
import { formatDateTime } from "../../util";
import TableId from "../TableId";

export default function AuditLog() {
  const { auditLogEntries } = useAuditLogEntries();

  // TODO: show additional details in modal?

  const rows = auditLogEntries?.map((entry) => (
    <Table.Tr key={entry.id}>
      <Table.Td>{entry.action}</Table.Td>
      <Table.Td>{entry.target_type} <TableId id={entry.target_id} /></Table.Td>
      <Table.Td>{entry.actor_type} <TableId id={entry.actor_id} /></Table.Td>
      <Table.Td>{formatDateTime(entry.occurred_at)}</Table.Td>
    </Table.Tr>
  ));

  return (
    <>
      <OrganizationHeader allowRename />
      <Title order={3} mb="md">
        Audit log
      </Title>
      <InfoAlert stateName="audit-log">
        The audit log provides a record of important actions and events that have occurred within your organization.
      </InfoAlert>

      <StyledTable headers={["Action", "Which", "Who", "Occurred at"]}>
        {rows}
      </StyledTable>
    </>
  );
}
