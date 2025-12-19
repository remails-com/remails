import { Progress, Stack, Text } from "@mantine/core";
import { useOrganizations } from "../../hooks/useOrganizations.ts";
import { formatDate, formatNumber } from "../../util.ts";
import StatCard from "./StatCard.tsx";

export default function Quota() {
  const { currentOrganization } = useOrganizations();

  if (!currentOrganization) {
    return null;
  }

  return (
    <StatCard
      title="Email quota"
      info="When your organization runs out of quota, emails will no longer be delivered. You can increase your quota by upgrading your subscription."
      footer={currentOrganization.quota_reset && `resets on ${formatDate(currentOrganization.quota_reset)}`}
    >
      <Stack gap={2} miw="80%">
        <Text ta="center">
          {formatNumber(currentOrganization.used_message_quota)}/{formatNumber(currentOrganization.total_message_quota)}{" "}
          used
        </Text>
        <Progress.Root transitionDuration={500} radius="xl" size="xl">
          <Progress.Section
            value={(currentOrganization.used_message_quota / currentOrganization.total_message_quota) * 100}
          ></Progress.Section>
        </Progress.Root>
      </Stack>
    </StatCard>
  );
}
