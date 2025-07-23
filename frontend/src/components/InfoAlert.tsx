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
            top: 0,
            right: 0,
          }}
          color="gray"
          onClick={() => setOpened(!opened)}
        />
      </Tooltip>
    );
  }

  return (
    <Alert icon={<IconInfoCircle />} color="gray" withCloseButton onClose={() => setOpened(false)} mb="sm">
      <Text>{children}</Text>
    </Alert>
  );
}
