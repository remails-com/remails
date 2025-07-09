import { useRemails } from "./useRemails.ts";

export function useProjects() {
  const {
    state: { projects, routerState },
    navigate,
  } = useRemails();
  const currentProject = projects?.find((p) => p.id === routerState.params.proj_id) || null;

  if (!currentProject && routerState.params.project_id) {
    navigate("not_found");
  }

  return { projects, currentProject };
}
