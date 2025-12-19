import { Text } from "@mantine/core";
import { formatNumber } from "../../util";
import StatCard from "./StatCard";
import { useOrganizations, useStatistics } from "../../hooks/useOrganizations";

export default function SuccessPercentage() {
  const { currentOrganization } = useOrganizations();

  const { monthly_statistics } = useStatistics();

  if (!currentOrganization) {
    return null;
  }

  const now = new Date();
  const months = [
    `${now.getUTCFullYear()}-${now.getMonth() + 1}-01`,
    `${now.getUTCFullYear()}-${now.getMonth() + 0}-01`,
    `${now.getUTCFullYear()}-${now.getMonth() - 1}-01`,
  ];

  let success = 0;
  let total = 0;
  for (const stat of monthly_statistics) {
    if (months.includes(stat.date)) {
      success += stat.statistics.delivered ?? 0;
      total += Object.values(stat.statistics).reduce((prev, cur) => (prev ?? 0) + (cur ?? 0)) ?? 0;
    }
  }

  return (
    <StatCard title="Successful delivery rate" footer={`over the last 3 months`}>
      <Text fz="xl" fw="bold" c="remails-red">
        {total > 0 ? `${formatNumber((success / total) * 100)}%` : "-"}
      </Text>
    </StatCard>
  );
}
