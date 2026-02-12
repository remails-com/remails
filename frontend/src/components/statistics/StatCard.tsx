import { Card, Center, Group, Stack, Text } from "@mantine/core";
import { ReactNode } from "react";
import InfoTooltip from "../InfoTooltip";

type StatCardProps = {
  title: ReactNode;
  info?: string; // for tooltip
  children: ReactNode; // main content
  footer?: ReactNode;
};

export default function StatCard({ title, info, children, footer }: StatCardProps) {
  return (
    <Card withBorder radius="md" shadow="sm" miw={220}>
      <Stack gap="sm" mih={120}>
        <Group gap="xs">
          <Text fw={700}>{title}</Text>
          {info && <InfoTooltip text={info} />}
        </Group>
        <Center flex={1}> {children}</Center>
        {footer && <Text>{footer}</Text>}
      </Stack>
    </Card>
  );
}
