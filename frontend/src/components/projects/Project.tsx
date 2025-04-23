import {useProjects} from "../../hooks/useProjects.ts";
import {Loader} from "../../Loader.tsx";

export function Project() {
  const {currentProject} = useProjects()

  if (!currentProject) {
    return <Loader/>;
  }

  return (
    <>
      ID: {currentProject.id}
<br/>
      Name: {currentProject.name}
    </>
  )
}