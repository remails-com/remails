import React from "react";
import { Alert, Text, Tooltip } from "@mantine/core";
import { IconHelp, IconInfoCircle } from "@tabler/icons-react";
import { useLocalStorage } from "@mantine/hooks";

export default function InfoAlert({ children, stateName }: { children: React.ReactNode; stateName: string }) {
  const [opened, setOpened] = useLocalStorage({
    key: `info-alert-${stateName}`,
    defaultValue: true,
  });

  if (!opened) {
    return (
      <Tooltip label={"Show more information"}>
        <IconHelp
          style={{
            position: "absolute",
            cursor: "pointer",
            top: "-11px",
            right: 0,
          }}
          color="gray"
          onClick={() => setOpened(!opened)}
        />
      </Tooltip>
    );
  }

  // This makes sure the font size remains consistent, while also preventing <p>'s in <p>'s
  if (typeof children === "string") {
    children = <Text>{children}</Text>;
  }

  return (
    <Alert icon={<IconInfoCircle />} color="gray" withCloseButton onClose={() => setOpened(false)} mb="sm">
      {children}
    </Alert>
  );
}
