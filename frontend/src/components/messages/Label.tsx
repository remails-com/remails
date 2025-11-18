import { Badge } from "@mantine/core";
import { useRemails } from "../../hooks/useRemails.ts";

function randomNumberFromStr(str: string): number {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    hash = (hash * 31 + str.charCodeAt(i)) | 0;
  }
  return hash >>> 0; // unsigned
}

export default function Label({ label }: { label: string }) {
  const { navigate } = useRemails();

  return (
    <Badge
      style={{
        cursor: "pointer",
      }}
      color={`hsl(${randomNumberFromStr(label) % 360}, 68%, 34%)`}
      onClick={(e) => {
        e.stopPropagation();
        navigate("projects.project.messages", {
          labels: label,
        });
      }}
    >
      {label}
    </Badge>
  );
}
