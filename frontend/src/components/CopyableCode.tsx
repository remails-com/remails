import {
  ActionIcon,
  Code,
  CodeProps,
  CSSProperties,
  Input,
  InputWrapperProps,
  Tooltip,
  useComputedColorScheme,
} from "@mantine/core";
import { useClipboard } from "@mantine/hooks";
import { IconCheck, IconCopy } from "@tabler/icons-react";
import React from "react";

interface CopyableCodeProps {
  children: string;
  label?: React.ReactNode;
  props?: InputWrapperProps;
  p?: CodeProps["p"];
}

export function CopyableCode({ children, label, props, p }: CopyableCodeProps) {
  const clipboard = useClipboard({ timeout: 1000 });

  const computedColorScheme = useComputedColorScheme();

  const style: CSSProperties = {
    // match background color of other input elements
    backgroundColor: computedColorScheme == "light" ? "var(--mantine-color-gray-1)" : "var(--mantine-color-dark-5)",

    // wrap long codes anywhere
    wordWrap: "break-word",
    whiteSpace: "pre-wrap",
    wordBreak: "break-all",

    cursor: "pointer",
  };

  return (
    <Input.Wrapper label={label} {...props}>
      <Tooltip label={clipboard.copied ? "Copied!" : "Click to copy"}>
        <Code p={p} block style={style} onClick={() => clipboard.copy(children)}>
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
          {children}
        </Code>
      </Tooltip>
    </Input.Wrapper>
  );
}
