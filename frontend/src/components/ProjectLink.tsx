import { Group } from "@mantine/core";
import { useProjectWithId } from "../hooks/useProjects";
import { Link } from "../Link";
import { IconServer } from "@tabler/icons-react";

interface ProjectLinkProps {
  project_id: string;
  size?: "md" | "sm";
}

export default function ProjectLink({ project_id, size }: ProjectLinkProps) {
  const project_name = useProjectWithId(project_id)?.name;

  return (
    <Link to={"projects.project"} params={{ proj_id: project_id }} style={{ size }}>
      <Group gap="0.4em">
        <IconServer size={size == "sm" ? 20 : 24} /> {project_name ?? project_id}
      </Group>
    </Link>
  );
}
