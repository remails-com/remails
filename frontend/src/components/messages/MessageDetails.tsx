import { useMessages } from "../../hooks/useMessages.ts";
import {
  Badge,
  Group,
  MantineSpacing,
  Paper,
  SegmentedControl,
  StyleProp,
  Table,
  Text,
  Title,
  Tooltip,
} from "@mantine/core";
import { ReactElement, useState } from "react";
import { Loader } from "../../Loader.tsx";
import { DeliveryStatus, Message, MessageMetadata } from "../../types.ts";
import { formatDateTime } from "../../util.ts";
import { IconCheck, IconClock, IconHelp, IconPaperclip, IconX } from "@tabler/icons-react";

export function getFullStatusDescription(message: MessageMetadata) {
  if (message.status == "Delivered") {
    return `Delivered ${message.reason}`;
  } else {
    return (
      message.status +
      (message.reason ? `: ${message.reason}` : "") +
      (message.retry_after ? `, retrying after ${formatDateTime(message.retry_after)}` : "") +
      (message.attempts > 1 ? ` (${message.attempts} attempts)` : "")
    );
  }
}

const deliveryStatus: {
  [key in DeliveryStatus["type"]]: { color: string; icon?: ReactElement };
} = {
  NotSent: { color: "secondary", icon: undefined },
  Success: { color: "green", icon: <IconCheck size={16} /> },
  Reattempt: { color: "orange", icon: <IconClock size={16} /> },
  Failed: { color: "red", icon: <IconX size={16} /> },
};

export function renderRecipients(
  message: MessageMetadata,
  ml?: StyleProp<MantineSpacing>,
  mr?: StyleProp<MantineSpacing>
) {
  return message.recipients.map((recipient: string) => {
    const status = message.delivery_status[recipient] ?? { type: "NotSent" };

    let tooltip = "Message not (yet) sent";
    if (status.type == "Failed") {
      tooltip = "Permanent failure";
    } else if (status.type == "Reattempt") {
      tooltip = "Temporary failure";
    } else if (status.type == "Success") {
      tooltip = `Delivered on ${status.delivered && formatDateTime(status.delivered)}`;
    }

    return (
      <Tooltip label={tooltip} key={recipient}>
        <Badge
          color={deliveryStatus[status.type].color}
          variant="light"
          ml={ml}
          mr={mr}
          rightSection={deliveryStatus[status.type].icon}
          tt="none"
          size="lg"
        >
          {recipient}
        </Badge>
      </Tooltip>
    );
  });
}

export default function MessageDetails() {
  const { currentMessage } = useMessages();
  const [displayMode, setDisplayMode] = useState("text");

  if (!currentMessage || !("message_data" in currentMessage && "truncated_raw_data" in currentMessage)) {
    return <Loader />;
  }

  const completeMessage = currentMessage as unknown as Message;

  const subject = completeMessage.message_data.subject;
  const text_body = completeMessage.message_data.text_body;
  const raw = completeMessage.truncated_raw_data;

  const recipients = renderRecipients(completeMessage, undefined, "sm");

  const table_data = [
    { header: "From", value: completeMessage.from_email },
    {
      header: "Recipients",
      info: 'The recipients who will receive this message based on the "RCPT TO" SMTP header',
      value: recipients,
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
              leftSection={<IconPaperclip />}
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
      {subject ? (
        <Title>{subject}</Title>
      ) : (
        <Title c="dimmed" fs="italic">
          No Subject
        </Title>
      )}

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
      <SegmentedControl
        mt="sm"
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
