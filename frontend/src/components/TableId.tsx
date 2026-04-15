import { Text, Tooltip } from "@mantine/core";
import { useClipboard } from "@mantine/hooks";

export default function TableId({ id, name }: { id: string, name?: string }) {
  const clipboard = useClipboard({ timeout: 1000 });


  return (
    <Tooltip label={clipboard.copied ? "Copied!" : id}>
      <Text span c={name ? "" : "dimmed"} size="sm" onClick={() => clipboard.copy(id)} style={{ cursor: "pointer" }}>
        {name ?? id.substring(0, 8)}
      </Text>
    </Tooltip>
  );
}
