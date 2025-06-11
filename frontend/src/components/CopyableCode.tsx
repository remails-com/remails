import { ActionIcon, Code, Input, MantineSpacing, StyleProp, Tooltip, useComputedColorScheme } from "@mantine/core";
import { useClipboard } from "@mantine/hooks";
import { IconCheck, IconCopy } from "@tabler/icons-react";

interface CopyableCodeProps {
  children: string;
  label?: React.ReactNode;
  mt?: StyleProp<MantineSpacing>;
}

export function CopyableCode({ children, label, mt }: CopyableCodeProps) {
  const clipboard = useClipboard({ timeout: 1000 });

  const computedColorScheme = useComputedColorScheme();

  const style = {
    // match background color of other input elements
    backgroundColor: computedColorScheme == "light" ? "var(--mantine-color-gray-1)" : "var(--mantine-color-dark-5)",

    // wrap long codes anywhere
    "word-wrap": "anywhere",
    "white-space": "pre-wrap",
    "word-break": "break-all",
  };

  return (
    <Input.Wrapper mt={mt} label={label}>
      <Code block style={style}>
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
    </Input.Wrapper>
  );
}
