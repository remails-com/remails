import { Accordion, ActionIcon, Badge, Button, Code, Group, NativeSelect, Text, Tooltip } from "@mantine/core";
import { useMessages } from "../../hooks/useMessages.ts";
import { Loader } from "../../Loader";
import { formatDateTime } from "../../util";
import { useRemails } from "../../hooks/useRemails.ts";
import { IconArrowLeft, IconArrowRight, IconCheck, IconClock, IconEye, IconRefresh, IconX } from "@tabler/icons-react";
import { getFullStatusDescription, renderRecipients } from "./MessageDetails.tsx";
import { DateTimePicker } from "@mantine/dates";
import dayjs from "dayjs";
import { useState } from "react";

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

const LIMIT_DEFAULT = "10"; // should match MessageFilter's default in src/models/messages.rs

export function MessageLog() {
  const { messages } = useMessages();
  const [pages, setPages] = useState<string[]>([]);
  const [refreshing, setRefreshing] = useState(false);
  const {
    state: { routerState },
    navigate,
  } = useRemails();

  if (!messages) {
    return <Loader />;
  }

  const setFilter = (filter: "limit" | "status" | "before", value: string | null) => {
    navigate(routerState.name, { ...routerState.params, [filter]: value ?? "" });
  };

  function refresh() {
    if (!refreshing) {
      setRefreshing(true);
      // TODO: maybe there is a nicer way to refresh the data
      navigate(routerState.name, { ...routerState.params });
      setTimeout(() => setRefreshing(false), 1000);
    }
  }

  function setBeforeFromPicker(value: string | null) {
    setPages([]);
    setFilter("before", value ? dayjs(value).toISOString() : null);
  }

  function loadNewer() {
    if (pages.length <= 1) {
      setPages([]);
      setFilter("before", null);
      return;
    }
    const previousDate = pages[pages.length - 2];
    console.log("previousDate", previousDate);
    console.log("pages", pages.slice(0, pages.length - 1));
    setPages(pages.slice(0, pages.length - 1));
    setFilter("before", previousDate);
  }

  function loadOlder() {
    const lastDate = messages![messages!.length - 1].created_at;
    console.log("pages", [...pages, lastDate]);
    setPages([...pages, lastDate]);
    setFilter("before", lastDate);
  }

  const has_more_entries = messages.length > parseInt(routerState.params.limit || LIMIT_DEFAULT);

  const rows = messages.slice(0, has_more_entries ? -1 : undefined).map((message) => (
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
            onClick={() => navigate("projects.project.streams.stream.messages.message", { message_id: message.id })}
          >
            View Message
          </Button>
        </Group>
      </Accordion.Panel>
    </Accordion.Item>
  ));

  const navigation_buttons = (
    <>
      <Button
        variant="default"
        leftSection={<IconArrowLeft />}
        onClick={loadNewer}
        disabled={!routerState.params.before}
      >
        newer messages
      </Button>
      <Button
        variant="default"
        rightSection={<IconArrowRight />}
        onClick={loadOlder}
        disabled={!has_more_entries || messages.length == 0}
      >
        older messages
      </Button>
    </>
  );

  return (
    <>
      <Group justify="space-between" align="flex-end">
        <Group>
          <NativeSelect
            label="Message status"
            value={routerState.params.status}
            data={[
              { label: "Show all", value: "" },
              { group: "In progress", items: ["Processing", "Accepted"] },
              { group: "Waiting for retry", items: ["Held", "Reattempt"] },
              { group: "Not delivered", items: ["Rejected", "Failed"] },
              "Delivered",
            ]}
            onChange={(event) => setFilter("status", event.currentTarget.value)}
          />
          <DateTimePicker
            label="From before date"
            value={routerState.params.before}
            placeholder="Pick date and time"
            onChange={setBeforeFromPicker}
            clearable
          />
          <NativeSelect
            label="Show per page"
            value={routerState.params.limit || LIMIT_DEFAULT}
            data={["10", "20", "50", "100"]}
            onChange={(event) => setFilter("limit", event.currentTarget.value)}
          />
        </Group>
        <Group justify="center">
          {navigation_buttons}
          <ActionIcon variant="default" size="input-sm" onClick={refresh} disabled={refreshing}>
            <IconRefresh
              style={
                refreshing ? { rotate: "-360deg", transition: "rotate 1s" } : { rotate: "0deg", transition: "none" }
              }
            />
          </ActionIcon>
        </Group>
      </Group>
      {rows.length == 0 ? (
        <Text c="dimmed" mt="md">
          No messages found...
        </Text>
      ) : (
        <>
          <Accordion my="md" variant="separated">
            {rows}
          </Accordion>
          <Group justify="center">{navigation_buttons}</Group>
        </>
      )}
    </>
  );
}
