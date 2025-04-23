import {useEffect} from "react";
import {useRemails} from "./useRemails.ts";

export function useProjects() {
  const {state: {currentOrganization, projects, currentProject, params}, dispatch} = useRemails();

  useEffect(() => {
    if (!currentOrganization) {
      return
    }

    dispatch({type: 'load_projects'})

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
      if (currentProject?.id === params.proj_id) {
        return
      }

      const nextCurrentProject = projects.find((p) => p.id === params.proj_id);
      if (nextCurrentProject) {
        dispatch({type: "set_current_project", project: nextCurrentProject})
      }
    }
  }, [params.proj_id, projects]);

  return {projects, currentProject}
}