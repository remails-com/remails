import { Badge, Tooltip } from "@mantine/core";
import { useRemails } from "../../hooks/useRemails.ts";

function randomNumberFromStr(str: string): number {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    hash = (hash * 31 + str.charCodeAt(i)) | 0;
  }
  return hash >>> 0; // unsigned
}

export default function Label({ label, clickable }: { label: string; clickable?: boolean }) {
  const { navigate } = useRemails();

  const r = randomNumberFromStr(label);
  const h = r % 360;
  const s = (Math.floor(r / 360) % 60) + 20;
  const l = (Math.floor(r / 360 / 60) % 60) + 20;

  return (
    <Tooltip label="Click to filter by this label" disabled={!clickable}>
      <Badge
        style={{
          cursor: "pointer",
        }}
        color={`hsl(${h}, ${s}%, ${l}%)`}
        onClick={(e) => {
          if (clickable) {
            e.stopPropagation();
            navigate("projects.project.messages", {
              labels: label,
            });
          }
        }}
        autoContrast
      >
        {label}
      </Badge>
    </Tooltip>
  );
}
