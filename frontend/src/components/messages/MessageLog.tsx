import {ActionIcon, Badge, Table, Text, Tooltip} from "@mantine/core";
import {useMessages} from "../../hooks/useMessages.ts";
import {Loader} from "../../Loader";
import {formatDateTime} from "../../util";
import {useRemails} from "../../hooks/useRemails.ts";
import {IconEye} from "@tabler/icons-react";

export function MessageLog() {
  const {messages} = useMessages();
  const {navigate} = useRemails();

  if (!messages) {
    return <Loader/>;
  }

  const rows = messages.map((message) => (
    <Table.Tr key={message.id}>
      <Table.Td>
        <Tooltip label={message.id}>
          <Text fz="sm">{message.id.slice(0, 8)}</Text>
        </Tooltip>
      </Table.Td>
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
      <Table.Td>
        <ActionIcon size="lg"
                    onClick={() => navigate('projects.project.streams.stream.message-log.message', {message_id: message.id})}>
          <IconEye/>
        </ActionIcon>
      </Table.Td>
    </Table.Tr>
  ));

  return (
    <Table>
      <Table.Thead>
        <Table.Tr>
          <Table.Th>ID</Table.Th>
          <Table.Th>Created</Table.Th>
          <Table.Th>From</Table.Th>
          <Table.Th>Recipients</Table.Th>
          <Table.Th>Status</Table.Th>
          <Table.Th/>
        </Table.Tr>
      </Table.Thead>
      <Table.Tbody>{rows}</Table.Tbody>
    </Table>
  );
}