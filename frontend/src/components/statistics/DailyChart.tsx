import { Card, Group, MultiSelect, Stack, Text } from "@mantine/core";
import { useOrganizations, useStatistics } from "../../hooks/useOrganizations";
import { AreaChart } from "@mantine/charts";
import { EmailStatus } from "../../types";
import { useState } from "react";
import { useProjects } from "../../hooks/useProjects";
import { ALL_EMAIL_STATUSES, STATUS_SERIES } from "./statuses";
import { formatDate } from "../../util";

type Stats = Record<EmailStatus, number> & { day: number };

// generates day keys, e.g. "2025-12-19"
const dayFormatter = new Intl.DateTimeFormat("en-CA", {
  timeZone: "UTC",
  year: "numeric",
  month: "2-digit",
  day: "2-digit",
});

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

  const [statusFilter, setStatusFilter] = useState<EmailStatus[]>([]);
  const [projectFilter, setProjectFilter] = useState<string[]>([]);

  if (!currentOrganization) {
    return null;
  }

  const data: Record<string, Stats> = {};

  // initialize past 30 days
  const now = new Date();
  const today = new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate()));
  for (let i = 0; i < 30; i++) {
    const d = new Date(today);
    d.setUTCDate(d.getUTCDate() - i);
    const timestamp = d.getTime();
    data[dayFormatter.format(d)] = getEmptyStats(timestamp);
  }

  for (const stat of daily_statistics) {
    if (projectFilter.length == 0 || projectFilter.includes(stat.project_id)) {
      for (const status of statusFilter.length > 0 ? statusFilter : ALL_EMAIL_STATUSES) {
        data[stat.date][status] += stat.statistics[status] ?? 0;
      }
    }
  }

  const sorted_data = Object.values(data);
  sorted_data.sort((a, b) => a.day - b.day);

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
              label="Email status"
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
              onChange={(status) => setStatusFilter(status.map((s) => s.toLowerCase() as EmailStatus))}
              maxDropdownHeight={400}
              clearable
              searchable
            />
          </Group>
        </Group>
        <AreaChart
          h={320}
          data={sorted_data}
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
