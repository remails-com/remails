import { RemailsError } from "../error/error.ts";
import { useSelector } from "./useSelector.ts";

export function useProjects() {
  const projects = useSelector((state) => state.projects || []);
  const routerState = useSelector((state) => state.routerState);
  const currentProject = projects?.find((p) => p.id === routerState.params.proj_id) ?? null;

  if (!currentProject && routerState.params.proj_id) {
    throw new RemailsError(`Could not find project with ID ${routerState.params.proj_id}`, 404);
  }

  return { projects, currentProject };
}

export function useProjectWithId(project_id: string | null) {
  const projects = useSelector((state) => state.projects || []);

  return project_id ? (projects?.find((p) => p.id === project_id) ?? null) : null;
}
