import { Card, Center, Group, Stack, Text, ThemeIcon, Tooltip } from "@mantine/core";
import { IconInfoCircle } from "@tabler/icons-react";
import { ReactNode } from "react";

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
          {info && (
            <Tooltip label={info}>
              <ThemeIcon variant="transparent" c="dimmed" size="sm">
                <IconInfoCircle />
              </ThemeIcon>
            </Tooltip>
          )}
        </Group>
        <Center flex={1}> {children}</Center>
        {footer && <Text>{footer}</Text>}
      </Stack>
    </Card>
  );
}
