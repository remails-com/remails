import { Accordion, Badge, Button, Code, Group, Text, Tooltip } from "@mantine/core";
import { useMessages } from "../../hooks/useMessages.ts";
import { Loader } from "../../Loader";
import { formatDateTime } from "../../util";
import { useRemails } from "../../hooks/useRemails.ts";
import { IconCheck, IconClock, IconEye, IconX } from "@tabler/icons-react";
import { getFullStatusDescription, renderRecipients } from "./MessageDetails.tsx";

function statusIcons(status: string) {
  if (status == "Processing" || status == "Accepted") {
    return <IconClock color="gray" />;
  } else if (status == "Held" || status == "Reattempt") {
    return <IconClock color="orange" />;
  } else if (status == "Rejected" || status == "Failed") {
    return <IconX color="red" />;
  } else if (status == "Delivered") {
    return <IconCheck color="green" />;
  }
  return <IconX color="gray" />;
}

export function MessageLog() {
  const { messages } = useMessages();
  const { navigate } = useRemails();

  if (!messages) {
    return <Loader />;
  }

  const rows = messages.map((message) => (
    <Accordion.Item key={message.id} value={message.id}>
      <Accordion.Control icon={statusIcons(message.status)}>
        {formatDateTime(message.created_at)}: message from
        <Badge color="secondary" size="lg" variant="light" mx="xs" tt="none">
          {message.from_email}
        </Badge>
        to
        {renderRecipients(message, "sm")}
      </Accordion.Control>
      <Accordion.Panel>
        <Text>
          Status: <span style={{ fontStyle: "italic" }}>{getFullStatusDescription(message)}</span>
        </Text>
        <Group justify="space-between" align="end">
          <Text fz="sm" c="dimmed">
            Message ID:{" "}
            <Tooltip label={message.id}>
              <Code>{message.id.slice(0, 8)}</Code>
            </Tooltip>
          </Text>
          <Button
            leftSection={<IconEye />}
            variant="light"
            size="xs"
            onClick={() => navigate("projects.project.streams.stream.message-log.message", { message_id: message.id })}
          >
            View Message
          </Button>
        </Group>
      </Accordion.Panel>
    </Accordion.Item>
  ));

  return <Accordion variant="separated">{rows}</Accordion>;
}
