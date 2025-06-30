import { useRemails } from "./useRemails.ts";

export function useProjects() {
  const {
    state: { projects, routerState },
  } = useRemails();
  const currentProject = projects?.find((p) => p.id === routerState.params.proj_id) || null;

  return { projects, currentProject };
}
