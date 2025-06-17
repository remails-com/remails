import { Text, Tooltip } from "@mantine/core";
import { useRemails } from "../hooks/useRemails";
import { formatDateTime } from "../util";

export function VersionInfo() {
  const {
    state: { config },
  } = useRemails();

  const version = config?.version;

  // In staging/production, the version is formatted as e.g. "1750084590-c164dbe"
  if (version && version.indexOf("-") > 0) {
    const [timestampStr, hash] = version.split("-", 2);
    const timestamp = parseInt(timestampStr);
    if (!isNaN(timestamp) && hash) {
      const date = formatDateTime(timestamp * 1000);
      return (
        <Tooltip label={`Updated ${date}`}>
          <Text c="dimmed" size="sm">
            {config?.environment} ({hash})
          </Text>
        </Tooltip>
      );
    }
  }

  return (
    <Text c="dimmed" size="sm">
      {config?.environment} ({version})
    </Text>
  );
}
