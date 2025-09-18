import { Card, Group, Progress, Text, Tooltip } from "@mantine/core";
import { useOrganizations } from "../../hooks/useOrganizations.ts";
import { formatDate, formatNumber } from "../../util.ts";

export default function Quota() {
  const { currentOrganization } = useOrganizations();

  if (!currentOrganization) {
    return null;
  }

  return (
    <Group>
      <Card withBorder radius="md" shadow="sm">
        <Text fz="sm" tt="uppercase" fw={700} ta="center">
          Used message quota
        </Text>
        <Tooltip
          events={{ touch: true, hover: true, focus: false }}
          label={`${formatNumber(currentOrganization.used_message_quota)} of ${formatNumber(currentOrganization.total_message_quota)}`}
        >
          <Progress.Root my="sm" transitionDuration={500} radius="xl" size="xl">
            <Progress.Section
              value={(currentOrganization.used_message_quota / currentOrganization.total_message_quota) * 100}
            ></Progress.Section>
          </Progress.Root>
        </Tooltip>
        {currentOrganization.quota_reset && (
          <Text>
            Resets on{" "}
            <Text span c={"remails-red"}>
              {formatDate(currentOrganization.quota_reset)}
            </Text>
          </Text>
        )}
      </Card>
    </Group>
  );
}
