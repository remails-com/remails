import { useMessages } from "../../hooks/useMessages.ts";
import { Badge, Group, Paper, SegmentedControl, Table, Text, Tooltip } from "@mantine/core";
import { useState } from "react";
import { Loader } from "../../Loader.tsx";
import { Message, MessageMetadata } from "../../types.ts";
import { formatDateTime, is_in_the_future } from "../../util.ts";
import { IconHelp, IconMessage, IconPaperclip } from "@tabler/icons-react";
import MessageRetryButton from "./MessageRetryButton.tsx";
import MessageDeleteButton from "./MessageDeleteButton.tsx";
import { Recipients } from "./Recipients.tsx";
import EntityHeader from "../EntityHeader.tsx";

export function getFullStatusDescription(message: MessageMetadata) {
  if (message.status == "Delivered") {
    return `Delivered ${message.reason}`;
  } else {
    let s = message.status;

    if (message.reason) {
      s += `: ${message.reason}`;
    }

    if (message.retry_after) {
      const retry_after_formatted = is_in_the_future(message.retry_after)
        ? "after " + formatDateTime(message.retry_after)
        : "soon";
      s += `, retrying ${retry_after_formatted}`;
    }

    if (message.attempts > 1) {
      s += ` (attempt ${message.attempts} of ${message.max_attempts})`;
    }

    return s;
  }
}

export default function MessageDetails() {
  const { currentMessage, updateMessage } = useMessages();
  const [displayMode, setDisplayMode] = useState("text");

  if (!currentMessage || !("message_data" in currentMessage && "truncated_raw_data" in currentMessage)) {
    return <Loader />;
  }

  const completeMessage = currentMessage as unknown as Message;

  const subject = completeMessage.message_data.subject;
  const text_body = completeMessage.message_data.text_body;
  const raw = completeMessage.truncated_raw_data;

  const table_data = [
    { header: "From", value: completeMessage.from_email },
    {
      header: "Recipients",
      info: 'The recipients who will receive this message based on the "RCPT TO" SMTP header',
      value: <Recipients message={completeMessage} mr="sm" />,
    },
    {
      header: "Date",
      info: "The time mentioned in the Date header of the message",
      value: completeMessage.message_data.date ? (
        formatDateTime(completeMessage.message_data.date)
      ) : (
        <Text c="dimmed" fs="italic">
          Message does not contain a Date header
        </Text>
      ),
    },
    {
      header: "Created",
      info: "The time that remails received this message",
      value: formatDateTime(completeMessage.created_at),
    },
    {
      header: "Total size",
      info: "The size of the whole message",
      value: completeMessage.raw_size,
    },
    {
      header: "Status",
      value: getFullStatusDescription(completeMessage),
    },
    {
      header: "Attachments",
      value:
        completeMessage.message_data.attachments.length === 0 ? (
          <Text c="dimmed" fs="italic">
            Message has no attachments
          </Text>
        ) : (
          completeMessage.message_data.attachments.map((attachment, index) => (
            <Badge
              key={`${attachment.filename}-${index}`}
              radius="xs"
              variant="light"
              size="lg"
              mr="xs"
              leftSection={<IconPaperclip size={18} />}
              rightSection={<Text fz="xs">{attachment.size}</Text>}
            >
              {attachment.filename}
            </Badge>
          ))
        ),
    },
  ];

  return (
    <>
      <EntityHeader name={subject ?? "no subject set"} entityType="Message" Icon={IconMessage} />

      <Table variant="vertical" layout="fixed" withTableBorder mt="sm">
        <Table.Tbody>
          {table_data.map((row) => (
            <Table.Tr key={row.header}>
              <Table.Th w="160">
                <Group justify="space-between">
                  <Text span mr="sm">
                    {row.header}
                  </Text>
                  {row.info && (
                    <Tooltip label={row.info} events={{ hover: true, touch: true, focus: false }}>
                      <IconHelp size={22} stroke={2} />
                    </Tooltip>
                  )}
                </Group>
              </Table.Th>
              <Table.Td>{row.value}</Table.Td>
            </Table.Tr>
          ))}
        </Table.Tbody>
      </Table>

      <Group my="sm" justify="right">
        <MessageRetryButton message={currentMessage} updateMessage={updateMessage} />
        <MessageDeleteButton message={currentMessage} />
      </Group>

      <SegmentedControl
        value={displayMode}
        onChange={setDisplayMode}
        data={[
          { label: "Text", value: "text" },
          { label: "Raw", value: "raw" },
        ]}
      />
      <Paper shadow={"xl"} p="sm" withBorder>
        {displayMode === "text" &&
          (text_body ? (
            <Text style={{ whiteSpace: "pre-wrap" }}>{text_body}</Text>
          ) : (
            <Text c="dimmed" fs="italic">
              No plain text version provided
            </Text>
          ))}
        {displayMode === "raw" &&
          (raw ? (
            <>
              <Text ff="monospace" fz="sm" style={{ whiteSpace: "pre-wrap" }}>
                {raw}
              </Text>
              {completeMessage.is_truncated && (
                <Text c="dimmed" fs="italic">
                  Message truncated
                </Text>
              )}
            </>
          ) : (
            <Text c="dimmed" fs="italic">
              Failed to load raw message data
            </Text>
          ))}
      </Paper>
    </>
  );
}
