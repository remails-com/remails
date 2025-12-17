import { Card, Group, MultiSelect, Stack, Text } from "@mantine/core";
import { useOrganizations, useStatistics } from "../../hooks/useOrganizations";
import { BarChart } from "@mantine/charts";
import { MessageStatus } from "../../types";
import { useState } from "react";
import { useProjects } from "../../hooks/useProjects";

const ALL_MESSAGE_STATUSES: MessageStatus[] = [
  "accepted",
  "delivered",
  "failed",
  "held",
  "processing",
  "reattempt",
  "rejected",
];

export default function PerMonthChart() {
  const { currentOrganization } = useOrganizations();
  const { projects } = useProjects();
  const { statistics } = useStatistics();

  const [statusFilter, setStatusFilter] = useState<MessageStatus[]>([]);
  const [projectFilter, setProjectFilter] = useState<string[]>([]);

  if (!currentOrganization || !statistics) {
    return null;
  }

  const data: Record<string, Record<MessageStatus, number> & { month: string }> = {};
  for (const stat of statistics) {
    if (projectFilter.length == 0 || projectFilter.includes(stat.project_id)) {
      data[stat.month] ??= {
        month: stat.month.substring(0, 7), // remove day
        accepted: 0,
        delivered: 0,
        failed: 0,
        held: 0,
        processing: 0,
        reattempt: 0,
        rejected: 0,
      };

      for (const status of statusFilter.length > 0 ? statusFilter : ALL_MESSAGE_STATUSES) {
        data[stat.month][status] += stat.statistics[status] ?? 0;
      }
    }
  }

  const sorted_data = Object.values(data);
  sorted_data.sort((a, b) => a.month.localeCompare(b.month));

  const series: { name: MessageStatus; color: string }[] = [
    { name: "accepted", color: "gray.6" },
    { name: "delivered", color: "teal.6" },
    { name: "failed", color: "red.6" },
    { name: "held", color: "orange.6" },
    { name: "processing", color: "blue.6" },
    { name: "reattempt", color: "yellow.6" },
    { name: "rejected", color: "grape.6" },
  ];

  return (
    <Card withBorder radius="md" shadow="sm" w="100%" miw={220}>
      <Stack gap="xl" h="100%">
        <Group gap="xs" justify="space-between" align="top">
          <Text fw={700} fz="lg">
            {"Emails sent per month"}
          </Text>
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
        <BarChart
          h={320}
          data={sorted_data}
          dataKey="month"
          type="stacked"
          series={series.filter((series) => sorted_data.some((dataPoint) => dataPoint[series.name] > 0))}
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
