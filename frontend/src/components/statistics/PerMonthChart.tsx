import { Card, Group, MultiSelect, Stack, Text } from "@mantine/core";
import { useOrganizations, useStatistics } from "../../hooks/useOrganizations";
import { AreaChart } from "@mantine/charts";
import { MessageStatus } from "../../types";
import { useState } from "react";
import { useProjects } from "../../hooks/useProjects";
import { formatMonth } from "../../util";
import { ALL_MESSAGE_STATUSES, STATUS_SERIES } from "./statuses";

export default function PerMonthChart() {
  const { currentOrganization } = useOrganizations();
  const { projects } = useProjects();
  const { monthly_statistics } = useStatistics();

  const [statusFilter, setStatusFilter] = useState<MessageStatus[]>([]);
  const [projectFilter, setProjectFilter] = useState<string[]>([]);

  if (!currentOrganization) {
    return null;
  }

  const data: Record<string, Record<MessageStatus, number> & { month: number }> = {};
  for (const stat of monthly_statistics) {
    if (projectFilter.length == 0 || projectFilter.includes(stat.project_id)) {
      data[stat.date] ??= {
        month: new Date(stat.date).getTime(),
        accepted: 0,
        delivered: 0,
        failed: 0,
        held: 0,
        processing: 0,
        reattempt: 0,
        rejected: 0,
      };

      for (const status of statusFilter.length > 0 ? statusFilter : ALL_MESSAGE_STATUSES) {
        data[stat.date][status] += stat.statistics[status] ?? 0;
      }
    }
  }

  const sorted_data = Object.values(data);
  sorted_data.sort((a, b) => a.month - b.month);

  return (
    <Card withBorder radius="md" shadow="sm" w="100%" miw={220}>
      <Stack gap="xl" h="100%">
        <Group gap="xs" justify="space-between" align="top">
          <Stack gap={0}>
            <Text fw={700} fz="lg">
              Emails sent per month
            </Text>
            <Text>since {formatMonth(sorted_data.length > 0 ? sorted_data[0].month : new Date())}</Text>
          </Stack>
          <Group>
            <MultiSelect
              label="Project"
              placeholder="Any project"
              value={projectFilter}
              data={projects.map((p) => ({ value: p.id, label: p.name }))}
              onChange={(projects) => setProjectFilter(projects)}
              clearable
              searchable
            />
            <MultiSelect
              label="Message status"
              placeholder="Any status"
              value={statusFilter}
              data={[
                { value: "delivered", label: "Delivered" },
                {
                  group: "In progress",
                  items: ["Processing", "Accepted"].map((i) => ({ value: i.toLowerCase(), label: i })),
                },
                {
                  group: "Waiting for retry",
                  items: ["Held", "Reattempt"].map((i) => ({ value: i.toLowerCase(), label: i })),
                },
                {
                  group: "Not delivered",
                  items: ["Rejected", "Failed"].map((i) => ({ value: i.toLowerCase(), label: i })),
                },
              ]}
              onChange={(status) => setStatusFilter(status.map((s) => s.toLowerCase() as MessageStatus))}
              maxDropdownHeight={400}
              clearable
              searchable
            />
          </Group>
        </Group>
        <AreaChart
          h={320}
          data={sorted_data}
          dataKey="month"
          xAxisProps={{
            tickFormatter: (ts) => formatMonth(ts),
          }}
          tooltipProps={{
            labelFormatter: (ts) => formatMonth(ts),
          }}
          series={STATUS_SERIES.filter((series) => sorted_data.some((dataPoint) => dataPoint[series.name] > 0))}
          withLegend
          legendProps={{ verticalAlign: "bottom" }}
          referenceLines={[
            {
              y: currentOrganization.total_message_quota,
              color: "red.5",
              label: "Quota",
              labelPosition: "insideTopRight",
            },
          ]}
        />
      </Stack>
    </Card>
  );
}
