import {useRemails} from "./useRemails.ts";

export function useProjects() {
  const {state: {projects, pathParams}} = useRemails();
  const currentProject = projects?.find((p) => p.id === pathParams.proj_id) || null;

  return {projects, currentProject}
}
