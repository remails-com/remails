import { useMessages } from "../../hooks/useMessages.ts";
import { Badge, Group, Paper, SegmentedControl, Table, Text, Title, Tooltip } from "@mantine/core";
import { useState } from "react";
import { Loader } from "../../Loader.tsx";
import { Message } from "../../types.ts";
import { formatDateTime } from "../../util.ts";
import { IconCheck, IconHelp, IconPaperclip, IconX } from "@tabler/icons-react";

export default function MessageDetails() {
  const { currentMessage } = useMessages();
  const [displayMode, setDisplayMode] = useState('text');


  if (!currentMessage || !('message_data' in currentMessage && 'truncated_raw_data' in currentMessage)) {
    return <Loader />
  }

  const completeMessage = currentMessage as unknown as Message;

  const subject = completeMessage.message_data.subject;
  const text_body = completeMessage.message_data.text_body;
  const raw = completeMessage.truncated_raw_data;

  const to = completeMessage.delivery_status.length > 0 ? completeMessage.delivery_status
    .map((status) => <Badge
      key={status.receiver}
      color={status.status == "Success" ? "green" : "red"}
      variant="light"
      mr="sm"
      rightSection={status.status == "Success" ? <IconCheck size={18} /> : <IconX size={18} />}
    >
      {status.receiver}
    </Badge>) : completeMessage
      .recipients
      .map((recipient: string) => <Badge
        key={recipient}
        color="secondary"
        variant="light"
        mr="sm">
        {recipient}
      </Badge>);

  const table_data = [
    { header: 'From', value: completeMessage.from_email },
    {
      header: 'To',
      value: to
    },
    {
      header: 'Date',
      info: 'The time mentioned in the Date header of the message',
      value: completeMessage.message_data.date ? formatDateTime(completeMessage.message_data.date) :
        <Text c="dimmed" fs="italic">Message does not contain a Date header</Text>
    },
    {
      header: 'Created',
      info: 'The time that remails received this message',
      value: formatDateTime(completeMessage.created_at)
    },
    {
      header: 'Total size',
      info: 'The size of the whole message',
      value: completeMessage.raw_size,
    },
    {
      header: 'Status',
      value: completeMessage.status + (completeMessage.reason ? ` (${completeMessage.reason})` : ''),
    },
    {
      header: 'Attachments',
      value: completeMessage.message_data.attachments.length === 0 ?
        <Text c="dimmed" fs="italic">Message has no attachments</Text>
        : completeMessage.message_data.attachments.map((attachment, index) => (
          <Badge key={`${attachment.filename}-${index}`}
            radius="xs"
            variant="light"
            size="lg"
            mr="xs"
            leftSection={<IconPaperclip />}
            rightSection={<Text fz="xs">{attachment.size}</Text>}
          >
            {attachment.filename}
          </Badge>
        ))
    }
  ]


  return (
    <>
      {subject ? <Title>{subject}</Title> :
        <Title c="dimmed" fs="italic">No Subject</Title>}

      <Table variant="vertical" layout="fixed" withTableBorder mt='sm'>
        <Table.Tbody>
          {table_data.map((row) => (
            <Table.Tr key={row.header}>
              <Table.Th w="160">
                <Group justify="space-between">
                  <Text span mr="sm">
                    {row.header}
                  </Text>
                  {row.info &&
                    <Tooltip label={row.info} events={{ hover: true, touch: true, focus: false }}>
                      <IconHelp size={22} stroke={2} />
                    </Tooltip>
                  }
                </Group>
              </Table.Th>
              <Table.Td>{row.value}</Table.Td>
            </Table.Tr>
          ))}
        </Table.Tbody>
      </Table>
      <SegmentedControl
        mt="sm"
        value={displayMode}
        onChange={setDisplayMode}
        data={[
          { label: 'Text', value: 'text' },
          { label: 'Raw', value: 'raw' },
        ]} />
      <Paper shadow={"xl"} p='sm' withBorder>
        {displayMode === 'text' && (
          text_body ? <Text style={{ whiteSpace: 'pre-wrap' }}>{text_body}</Text> :
            <Text c="dimmed" fs="italic">No plain text version provided</Text>
        )
        }
        {displayMode === 'raw' && (
          raw ? <><Text ff='monospace' fz="sm" style={{ whiteSpace: 'pre-wrap' }}>{raw}</Text>
            {completeMessage.is_truncated &&
              <Text c="dimmed" fs="italic">Message truncated</Text>
            }
          </> :
            <Text c="dimmed" fs="italic">Failed to load raw message data</Text>
        )
        }
      </Paper>
    </>
  )
}