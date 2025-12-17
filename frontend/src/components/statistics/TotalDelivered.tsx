import { Text } from "@mantine/core";
import { formatDate, formatNumber } from "../../util";
import StatCard from "./StatCard";
import { useOrganizations, useStatistics } from "../../hooks/useOrganizations";

export default function TotalDelivered() {
  const { currentOrganization } = useOrganizations();

  const { statistics } = useStatistics();

  if (!currentOrganization || !statistics) {
    return null;
  }

  let total = 0;
  for (const stat of statistics) {
    total += stat.statistics.delivered ?? 0;
  }

  return (
    <StatCard title="Total emails delivered" footer={`since ${formatDate(currentOrganization.created_at)}`}>
      <Text fz="xl" fw="bold" c="remails-red">
        {formatNumber(total)}
      </Text>
    </StatCard>
  );
}
