import {useMessages} from "../../hooks/useMessages.ts";
import {Badge, Group, Paper, SegmentedControl, Table, Text, Title, Tooltip} from "@mantine/core";
import {useState} from "react";
import {Loader} from "../../Loader.tsx";
import {Message} from "../../types.ts";
import {formatDateTime} from "../../util.ts";
import {IconHelp, IconPaperclip} from "@tabler/icons-react";

export default function MessageDetails() {
  const {currentMessage} = useMessages();
  const [displayMode, setDisplayMode] = useState('html');


  if (!currentMessage || !('message_data' in currentMessage && 'raw_data' in currentMessage)) {
    return <Loader/>
  }

  const completeMessage = currentMessage as unknown as Message;

  const subject = completeMessage.message_data.subject;
  const text_body = completeMessage.message_data.text_body;
  const html_body = completeMessage.message_data.html_body;
  const raw = completeMessage.raw_data;

  const table_data = [
    {header: 'From', value: completeMessage.from_email},
    {
      header: 'To',
      value: completeMessage
        .recipients
        .map((recipient: string) => <Badge
          key={recipient}
          color="secondary"
          variant="light"
          mr="sm">
          {recipient}
        </Badge>)
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
      header: 'Attachments',
      value: completeMessage.message_data.attachments.length === 0 ?
        <Text c="dimmed" fs="italic">Message has no attachments</Text>
        : completeMessage.message_data.attachments.map((attachment, index) => (
          <Badge key={`${attachment.filename}-${index}`}
                 radius="xs"
                 variant="light"
                 size="lg"
                 mr="xs"
                 leftSection={<IconPaperclip/>}
                 rightSection={<Text fz="xs">{attachment.size}</Text>}
                 component="a"
                 download={attachment.filename}
                 href={`data:${attachment.mime};base64,${attachment.content}`}
          >
            {attachment.filename}
          </Badge>
        ))
    }
  ]


  // TODO fix automatic sizing of the iframe
  return (
    <>
      {subject ? <Title>{subject}</Title> :
        <Title c="dimmed" fs="italic">No Subject</Title>}

      <Table variant="vertical" layout="fixed" withTableBorder>
        <Table.Tbody>
          {table_data.map((row) => (
            <Table.Tr key={row.header}>
              <Table.Th w="160">
                <Group justify="space-between">
                  <Text span mr="sm">
                    {row.header}
                  </Text>
                  {row.info &&
                      <Tooltip label={row.info} events={{hover: true, touch: true, focus: false}}>
                          <IconHelp size={22} stroke={2}/>
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
          {label: 'HTML', value: 'html'},
          {label: 'Text', value: 'text'},
          {label: 'Raw', value: 'raw'},
        ]}/>
      <Paper shadow={"xl"} p='sm' withBorder>
        {displayMode === 'html' && (
          html_body ? <iframe sandbox='' srcDoc={html_body} style={{width: '100%', border: 'none'}}/> :
            <Text c="dimmed" fs="italic">No HTML version provided</Text>
        )
        }
        {displayMode === 'text' && (
          text_body ? <Text style={{whiteSpace: 'pre-wrap'}}>{text_body}</Text> :
            <Text c="dimmed" fs="italic">No plain text version provided</Text>
        )
        }
        {displayMode === 'raw' && (
          raw ? <Text ff='monospace' fz="sm" style={{whiteSpace: 'pre-wrap'}}>{raw}</Text> :
            <Text c="dimmed" fs="italic">Failed to load raw message data</Text>
        )
        }
      </Paper>
    </>
  )
}