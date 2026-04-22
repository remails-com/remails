import { useEmails } from "../../hooks/useEmails.ts";
import { Badge, Group, Paper, SegmentedControl, Table, Text, Tooltip } from "@mantine/core";
import { useState } from "react";
import { Loader } from "../../Loader.tsx";
import { EmailMetadata, isFullEmail } from "../../types.ts";
import { formatDateTime, is_in_the_future } from "../../util.ts";
import { IconHelp, IconMail, IconPaperclip } from "@tabler/icons-react";
import EmailRetryButton from "./EmailRetryButton.tsx";
import EmailDeleteButton from "./EmailDeleteButton.tsx";
import { Recipients } from "./Recipients.tsx";
import Header from "../Header.tsx";
import Label from "./Label.tsx";
import ProjectLink from "../ProjectLink.tsx";

export function getFullStatusDescription(email: EmailMetadata) {
  if (email.status == "delivered") {
    return `delivered ${email.reason}`;
  } else {
    let s = email.status;

    if (email.reason) {
      s += `: ${email.reason}`;
    }

    if (email.retry_after) {
      const retry_after_formatted = is_in_the_future(email.retry_after)
        ? "after " + formatDateTime(email.retry_after)
        : "soon";
      s += `, retrying ${retry_after_formatted}`;
    }

    if (email.attempts > 1) {
      s += ` (attempt ${email.attempts} of ${email.max_attempts})`;
    }

    return s;
  }
}

export default function EmailDetails() {
  const { currentEmail, updateEmail } = useEmails();
  const [displayMode, setDisplayMode] = useState("text");

  if (!isFullEmail(currentEmail)) {
    return <Loader />;
  }

  const subject = currentEmail.message_data.subject;
  const text_body = currentEmail.message_data.text_body;
  const raw = currentEmail.truncated_raw_data;

  const table_data = [
    {
      header: "Subject",
      value: subject ?? (
        <Text c="dimmed" fs="italic" fz="sm">
          No subject
        </Text>
      ),
    },
    { header: "From", value: currentEmail.from_email },
    { header: "Project", value: <ProjectLink project_id={currentEmail.project_id} size="sm" /> },
    {
      header: "Recipients",
      info: 'The recipients who will receive this email based on the "RCPT TO" SMTP header',
      value: <Recipients email={currentEmail} mr="sm" />,
    },
    {
      header: "Message ID",
      info: "The Message-ID email header is used to identify emails (e.g. used to send replies)",
      value: currentEmail.message_id_header,
    },
    {
      header: "Date",
      info: "The time mentioned in the Date header of the email",
      value: currentEmail.message_data.date ? (
        formatDateTime(currentEmail.message_data.date)
      ) : (
        <Text c="dimmed" fs="italic" fz="sm">
          Message does not contain a Date header
        </Text>
      ),
    },
    {
      header: "Created",
      info: "The time that remails received this email",
      value: formatDateTime(currentEmail.created_at),
    },
    {
      header: "Total size",
      info: "The size of the whole email",
      value: currentEmail.raw_size,
    },
    {
      header: "Status",
      value: getFullStatusDescription(currentEmail),
    },
    {
      header: "Attachments",
      value:
        currentEmail.message_data.attachments.length === 0 ? (
          <Text c="dimmed" fs="italic" fz="sm">
            Email has no attachments
          </Text>
        ) : (
          currentEmail.message_data.attachments.map((attachment, index) => (
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
      <Header
        name={subject ?? "No subject"}
        entityType="Email"
        Icon={IconMail}
        divider
        addendum={currentEmail.label ? <Label label={currentEmail.label} clickable /> : null}
      />

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
        <EmailRetryButton email={currentEmail} updateEmail={updateEmail} />
        <EmailDeleteButton email={currentEmail} />
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
              {currentEmail.is_truncated && (
                <Text c="dimmed" fs="italic">
                  Email truncated
                </Text>
              )}
            </>
          ) : (
            <Text c="dimmed" fs="italic">
              Failed to load raw email data
            </Text>
          ))}
      </Paper>
    </>
  );
}
