import {createContext, useContext, useEffect, useState} from "react";
import {Organization, Project,} from "../types";

export interface ProjectsContextProps {
  currentProject?: Project;
  setCurrentProject: (currentProject: Project) => void;
  projects: Project[];
  loading: boolean;
}

export const ProjectContext = createContext<ProjectsContextProps | null>(null);

export function useProjects(): ProjectsContextProps {
  const projects = useContext(ProjectContext);

  if (!projects) {
    throw new Error("useProject must be used within a ProjectProvider");
  }

  return projects;
}

export function useLoadProjects(currentOrganization?: Organization) {
  const [projects, setProjects] = useState<Project[]>([]);
  const [loading, setLoading] = useState(true);
  const [currentProject, setCurrentProject] = useState<Project | undefined>(undefined);

  useEffect(() => {
    setLoading(true);
    
    if (!currentOrganization) {
      return
    }

    fetch(`/api/organizations/${currentOrganization.id}/projects`)
      .then((res) => res.json())
      .then((data) => {
        if (Array.isArray(data)) {
          setProjects(data);
          setCurrentProject(data[0]);
        }
        setLoading(false);
      });
  }, [currentOrganization]);

  return {projects, loading, currentProject, setCurrentProject}
}