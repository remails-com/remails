import {useRemails} from "./useRemails.ts";

export function useProjects() {
  const {state: {projects, params}} = useRemails();
  const currentProject = projects?.find((p) => p.id === params.proj_id) || null;

  return {projects, currentProject}
}
