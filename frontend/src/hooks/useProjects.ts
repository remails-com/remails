import {useEffect} from "react";
import {useRemails} from "./useRemails.ts";
import {useRouter} from "./useRouter.ts";

export function useProjects() {
  const {state: {currentOrganization, projects, currentProject}, dispatch} = useRemails();
  const {params} = useRouter();

  useEffect(() => {
    dispatch({type: 'load_projects'})

    if (!currentOrganization) {
      return
    }

    fetch(`/api/organizations/${currentOrganization.id}/projects`)
      .then((res) => res.json())
      .then((data) => {
        if (Array.isArray(data)) {
          dispatch({type: 'set_projects', projects: data});
        }
      });
  }, [currentOrganization]);

  useEffect(() => {
    if (params.proj_id && projects) {
      const nextCurrentProject = projects.find((p) => p.id === params.proj_id);
      if (nextCurrentProject) {
        dispatch({type: "set_current_project", project: nextCurrentProject})
      }
    }
  }, [params.proj_id, projects]);

  return {projects, currentProject}
}