import { Accordion, ActionIcon, Badge, Box, Button, Code, Group, MultiSelect, NativeSelect, Text } from "@mantine/core";
import { useEmails } from "../../hooks/useEmails.ts";
import { Loader } from "../../Loader.tsx";
import { formatDateTime } from "../../util.ts";
import { useRemails } from "../../hooks/useRemails.ts";
import { IconArrowLeft, IconArrowRight, IconCheck, IconClock, IconEye, IconRefresh, IconX } from "@tabler/icons-react";
import { getFullStatusDescription } from "./EmailDetails.tsx";
import { DateTimePicker } from "@mantine/dates";
import dayjs from "dayjs";
import { useState } from "react";
import EmailDeleteButton from "./EmailDeleteButton.tsx";
import EmailRetryButton from "./EmailRetryButton.tsx";
import { Recipients } from "./Recipients.tsx";
import InfoAlert from "../InfoAlert.tsx";
import Label from "./Label.tsx";
import { EmailStatus } from "../../types.ts";
import OrganizationHeader from "../organizations/OrganizationHeader.tsx";
import ProjectLink from "../ProjectLink.tsx";
import InfoTooltip from "../InfoTooltip.tsx";

function statusIcons(status: EmailStatus) {
  if (status == "processing" || status == "accepted") {
    return <IconClock color="gray" />;
  } else if (status == "held" || status == "reattempt") {
    return <IconClock color="orange" />;
  } else if (status == "rejected" || status == "failed") {
    return <IconX color="red" />;
  } else if (status == "delivered") {
    return <IconCheck color="green" />;
  }
  return <IconX color="gray" />;
}

const LIMIT_DEFAULT = "10"; // should match MessageFilter's default in src/models/messages.rs

export function EmailOverview() {
  const { emails, updateEmail, labels } = useEmails();
  const [pages, setPages] = useState<string[]>([]);
  const [refreshing, setRefreshing] = useState(false);
  const {
    state: { routerState, nextRouterState },
    navigate,
  } = useRemails();

  const currentParams = nextRouterState?.params || routerState.params;

  if (!emails) {
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
    const lastDate = emails![emails!.length - 1].created_at;
    setPages([...pages, lastDate]);
    setFilter("before", lastDate);
  }

  const has_more_entries = emails.length > parseInt(currentParams.limit || LIMIT_DEFAULT);

  const rows = emails.slice(0, has_more_entries ? -1 : undefined).map((email) => (
    <Accordion.Item key={email.id} value={email.id}>
      <Accordion.Control icon={statusIcons(email.status)}>
        <Group gap={0} justify="space-between" align="center">
          <Box>
            Email from
            <Badge color="secondary" size="lg" variant="light" mx="xs" tt="none">
              {email.from_email}
            </Badge>
            to
            <Recipients email={email} ml="sm" />
          </Box>
          <Group>
            {email.label && <Label label={email.label} clickable />}
            <Text fz="xs" c="dimmed" mr="md">
              {formatDateTime(email.created_at)}
            </Text>
          </Group>
        </Group>
      </Accordion.Control>
      <Accordion.Panel>
        <Text>
          Status: <span style={{ fontStyle: "italic" }}>{getFullStatusDescription(email)}</span>
        </Text>
        {routerState.name == "emails" && (
          <Group gap="0.4em">
            Project: <ProjectLink project_id={email.project_id} />
          </Group>
        )}
        <Group justify="space-between" align="end">
          <Text fz="sm" c="dimmed">
            Message ID: {<Code>{email.message_id_header}</Code>}
          </Text>
          <Group>
            <EmailDeleteButton email={email} small />
            <EmailRetryButton email={email} updateEmail={updateEmail} small />
            <Button
              leftSection={<IconEye />}
              variant="light"
              size="xs"
              onClick={() =>
                navigate(routerState.name == "emails" ? "emails.email" : "projects.project.emails.email", {
                  email_id: email.id,
                })
              }
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
        disabled={!has_more_entries || emails.length == 0}
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
      {routerState.name == "emails" && <OrganizationHeader />}

      <InfoAlert stateName="messages">
        This page shows a list of all emails sent in this {routerState.name == "emails" ? "organization" : "project"}.
        Use it to check delivery status, inspect metadata, and troubleshoot issues. You’ll see timestamps, recipient
        addresses, and SMTP-level details for each message. Messages are automatically deleted after the rentention
        period set in the project settings.
      </InfoAlert>

      <Group justify="space-between" align="flex-end">
        <Group>
          <MultiSelect
            label={
              <Group gap={4} align="center">
                Label
                <InfoTooltip
                  size="xs"
                  text="Labels can be used to catagorize emails. Specify the label by setting the X-REMAILS-LABEL header or using the REST API."
                />
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
