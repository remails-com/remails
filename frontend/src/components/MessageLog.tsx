import { Badge, Table } from "@mantine/core";
import { useMessageLog } from "../hooks/useMessageLog";
import { Loader } from "../Loader";
import { formatDateTime } from "../util";

export function MessageLog() {
  const { messages, loading } = useMessageLog();

  if (loading) {
    return <Loader />;
  }

  const rows = messages.map((message) => (
    <Table.Tr key={message.id}>
      <Table.Td>{message.id}</Table.Td>
      <Table.Td>{formatDateTime(message.created_at)}</Table.Td>
      <Table.Td>
        <Badge color="secondary" size="lg" variant="light" mr="sm" tt="none">
          {message.from_email}
        </Badge>
      </Table.Td>
      <Table.Td>{message.recipients.map((recipient, index) => (
        <Badge key={`${recipient}-${index}`} color="secondary" size="lg" variant="light" mr="sm" tt="none">
          {recipient}
        </Badge>
      ))}</Table.Td>
      <Table.Td>{message.status}</Table.Td>
    </Table.Tr>
  ));

  return (
    <Table>
      <Table.Thead>
        <Table.Tr>
          <Table.Th>ID</Table.Th>
          <Table.Th>Date</Table.Th>
          <Table.Th>From</Table.Th>
          <Table.Th>Recipients</Table.Th>
          <Table.Th>Status</Table.Th>
        </Table.Tr>
      </Table.Thead>
      <Table.Tbody>{rows}</Table.Tbody>
    </Table>
  );
}