import {StreamsOverview} from "../streams/StreamsOverview.tsx";
import {Project} from "../../types.ts";

export interface ProjectDetailsProps {
  currentProject: Project;
}

export function ProjectDetails({currentProject}: ProjectDetailsProps) {
  return (
    <>
      ID: {currentProject.id}
      <br/>
      Name: {currentProject.name}
      <br/>
      <h2>Streams</h2>
      <StreamsOverview currentProject={currentProject}/>
    </>

  )
}