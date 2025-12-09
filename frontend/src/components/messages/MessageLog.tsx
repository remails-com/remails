import {
  Accordion,
  ActionIcon,
  Badge,
  Box,
  Button,
  Code,
  Group,
  MultiSelect,
  NativeSelect,
  Text,
  ThemeIcon,
  Tooltip,
} from "@mantine/core";
import { useMessages } from "../../hooks/useMessages.ts";
import { Loader } from "../../Loader";
import { formatDateTime } from "../../util";
import { useRemails } from "../../hooks/useRemails.ts";
import {
  IconArrowLeft,
  IconArrowRight,
  IconCheck,
  IconClock,
  IconEye,
  IconInfoCircle,
  IconRefresh,
  IconX,
} from "@tabler/icons-react";
import { getFullStatusDescription } from "./MessageDetails.tsx";
import { DateTimePicker } from "@mantine/dates";
import dayjs from "dayjs";
import { useState } from "react";
import MessageDeleteButton from "./MessageDeleteButton.tsx";
import MessageRetryButton from "./MessageRetryButton.tsx";
import { Recipients } from "./Recipients.tsx";
import InfoAlert from "../InfoAlert.tsx";
import Label from "./Label.tsx";

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
  const { messages, updateMessage, labels } = useMessages();
  const [pages, setPages] = useState<string[]>([]);
  const [refreshing, setRefreshing] = useState(false);
  const {
    state: { routerState, nextRouterState },
    navigate,
  } = useRemails();

  const currentParams = nextRouterState?.params || routerState.params;

  if (!messages) {
    return <Loader />;
  }

  const setFilter = (filter: "limit" | "status" | "before" | "labels", value: string | null) => {
    navigate(routerState.name, { ...routerState.params, [filter]: value ?? "" });
  };

  function refresh() {
    if (!refreshing) {
      setRefreshing(true);
      navigate(routerState.name, { ...routerState.params, force: "reload" });
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
    setPages(pages.slice(0, pages.length - 1));
    setFilter("before", previousDate);
  }

  function loadOlder() {
    const lastDate = messages![messages!.length - 1].created_at;
    setPages([...pages, lastDate]);
    setFilter("before", lastDate);
  }

  const has_more_entries = messages.length > parseInt(currentParams.limit || LIMIT_DEFAULT);

  const rows = messages.slice(0, has_more_entries ? -1 : undefined).map((message) => (
    <Accordion.Item key={message.id} value={message.id}>
      <Accordion.Control icon={statusIcons(message.status)}>
        <Group gap={0} justify="space-between" align="center">
          <Box>
            Email from
            <Badge color="secondary" size="lg" variant="light" mx="xs" tt="none">
              {message.from_email}
            </Badge>
            to
            <Recipients message={message} ml="sm" />
          </Box>
          <Group>
            {message.label && <Label label={message.label} clickable />}
            <Text fz="xs" c="dimmed" mr="md">
              {formatDateTime(message.created_at)}
            </Text>
          </Group>
        </Group>
      </Accordion.Control>
      <Accordion.Panel>
        <Text>
          Status: <span style={{ fontStyle: "italic" }}>{getFullStatusDescription(message)}</span>
        </Text>
        <Group justify="space-between" align="end">
          <Text fz="sm" c="dimmed">
            Message ID: {<Code>{message.message_id_header}</Code>}
          </Text>
          <Group>
            <MessageDeleteButton message={message} small />
            <MessageRetryButton message={message} updateMessage={updateMessage} small />
            <Button
              leftSection={<IconEye />}
              variant="light"
              size="xs"
              onClick={() => navigate("projects.project.emails.email", { message_id: message.id })}
            >
              View email
            </Button>
          </Group>
        </Group>
      </Accordion.Panel>
    </Accordion.Item>
  ));

  const navigation_buttons = (
    <>
      <Button
        variant="default"
        leftSection={<IconArrowLeft />}
        onClick={loadOlder}
        disabled={!has_more_entries || messages.length == 0}
      >
        older emails
      </Button>
      <Button
        variant="default"
        rightSection={<IconArrowRight />}
        onClick={loadNewer}
        disabled={!routerState.params.before}
      >
        newer emails
      </Button>
    </>
  );

  return (
    <>
      <InfoAlert stateName="messages">
        This page shows a list of all emails sent in this project. Use it to check delivery status, inspect metadata,
        and troubleshoot issues. You’ll see timestamps, recipient addresses, and SMTP-level details for each message.
      </InfoAlert>
      <Group justify="space-between" align="flex-end">
        <Group>
          <MultiSelect
            label={
              <Group gap={4} align="center">
                Label
                <Tooltip label="Labels can be used to catagorize emails. Specify the label by setting the X-REMAILS-LABEL header or using the REST API.">
                  <ThemeIcon variant="transparent" c="dimmed" size="xs">
                    <IconInfoCircle />
                  </ThemeIcon>
                </Tooltip>
              </Group>
            }
            placeholder="Pick labels"
            data={labels}
            value={currentParams.labels?.split(",").filter((l) => l.trim().length > 0) || []}
            searchable
            hidePickedOptions
            nothingFoundMessage="No labels found..."
            onChange={(labels) => setFilter("labels", labels.join(","))}
            renderOption={({ option }) => <Label label={option.value} />}
          />
          <MultiSelect
            label="Message status"
            placeholder="Pick status"
            value={currentParams.status?.split(",").filter((l) => l.trim().length > 0) || []}
            data={[
              "Delivered",
              { group: "In progress", items: ["Processing", "Accepted"] },
              { group: "Waiting for retry", items: ["Held", "Reattempt"] },
              { group: "Not delivered", items: ["Rejected", "Failed"] },
            ]}
            onChange={(status) => setFilter("status", status.join(","))}
            maxDropdownHeight={400}
          />
          <DateTimePicker
            label="Created before"
            value={currentParams.before}
            placeholder="Pick date and time"
            onChange={setBeforeFromPicker}
            clearable
          />
          <NativeSelect
            label="Show per page"
            value={currentParams.limit || LIMIT_DEFAULT}
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
          No emails found...
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
