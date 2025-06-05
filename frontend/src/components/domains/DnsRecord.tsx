import { ActionIcon, Code, Tooltip } from "@mantine/core";
import { useClipboard } from "@mantine/hooks";
import { IconCheck, IconCopy } from "@tabler/icons-react";

export function DnsRecord({ children }: { children: string }) {
  const clipboard = useClipboard({ timeout: 1000 });

  return (
    <Code block style={{ "word-wrap": "anywhere", "white-space": "pre-wrap", "word-break": "break-all" }} mt="xs">
      <Tooltip label={clipboard.copied ? "Copied" : "Copy"}>
        <ActionIcon
          variant="light"
          color={clipboard.copied ? "teal" : "blue"}
          onClick={() => clipboard.copy(children)}
          size="xs"
          aria-label="Copy"
          style={{ float: "right" }}
        >
          {clipboard.copied ? <IconCheck></IconCheck> : <IconCopy></IconCopy>}
        </ActionIcon>
      </Tooltip>
      {children}
    </Code>
  );
}
