import { Card, Group, MultiSelect, Stack, Text } from "@mantine/core";
import { useOrganizations, useStatistics } from "../../hooks/useOrganizations";
import { AreaChart } from "@mantine/charts";
import { MessageStatus } from "../../types";
import { useState } from "react";
import { useProjects } from "../../hooks/useProjects";
import { ALL_MESSAGE_STATUSES, STATUS_SERIES } from "./statuses";
import { formatDate } from "../../util";

type Stats = Record<MessageStatus, number> & { day: number };

function getEmptyStats(date: string | number | Date): Stats {
  return {
    day: new Date(date).getTime(),
    accepted: 0,
    delivered: 0,
    failed: 0,
    held: 0,
    processing: 0,
    reattempt: 0,
    rejected: 0,
  };
}

export default function DailyChart() {
  const { currentOrganization } = useOrganizations();
  const { projects } = useProjects();
  const { daily_statistics } = useStatistics();

  const [statusFilter, setStatusFilter] = useState<MessageStatus[]>([]);
  const [projectFilter, setProjectFilter] = useState<string[]>([]);

  if (!currentOrganization) {
    return null;
  }

  const data: Record<string, Stats> = {};
  for (const stat of daily_statistics) {
    if (projectFilter.length == 0 || projectFilter.includes(stat.project_id)) {
      data[stat.date] ??= getEmptyStats(stat.date);

      for (const status of statusFilter.length > 0 ? statusFilter : ALL_MESSAGE_STATUSES) {
        data[stat.date][status] += stat.statistics[status] ?? 0;
      }
    }
  }

  const sorted_data = Object.values(data);
  sorted_data.sort((a, b) => a.day - b.day);

  // fill missing dates
  const final_data: Stats[] = [];

  const current = new Date(sorted_data[0].day);
  const end = new Date(sorted_data[sorted_data.length - 1].day);

  let i = 0;

  while (current <= end) {
    const currentDayMs = current.getTime();

    if (i < sorted_data.length && sorted_data[i].day === currentDayMs) {
      final_data.push(sorted_data[i]);
      i++;
    } else {
      final_data.push(getEmptyStats(currentDayMs));
    }

    current.setUTCDate(current.getUTCDate() + 1);
  }

  return (
    <Card withBorder radius="md" shadow="sm" w="100%" miw={220}>
      <Stack gap="xl" h="100%">
        <Group gap="xs" justify="space-between" align="top">
          <Stack gap={0}>
            <Text fw={700} fz="lg">
              Emails sent per day
            </Text>
            <Text>for the past 30 days</Text>
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
          data={final_data}
          dataKey="day"
          xAxisProps={{
            type: "number",
            scale: "time",
            domain: ["auto", "auto"],
            tickFormatter: (ts) => formatDate(ts),
          }}
          tooltipProps={{
            labelFormatter: (ts) => formatDate(ts),
          }}
          series={STATUS_SERIES.filter((series) => sorted_data.some((dataPoint) => dataPoint[series.name] > 0))}
          withLegend
          legendProps={{ verticalAlign: "bottom" }}
        />
      </Stack>
    </Card>
  );
}
