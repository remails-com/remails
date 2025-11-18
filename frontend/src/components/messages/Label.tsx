import { Badge } from "@mantine/core";
import { useRemails } from "../../hooks/useRemails.ts";

export default function Label({ label }: { label: string }) {
  const { navigate } = useRemails();

  return (
    <Badge
      style={{
        cursor: "pointer",
      }}
      color="#FF246B"
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
