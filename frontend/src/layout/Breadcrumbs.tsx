import {Anchor, Breadcrumbs as MantineBreadcrumbs} from "@mantine/core";
import {useProjects} from "../hooks/useProjects.ts";
import {useRemails} from "../hooks/useRemails.ts";
import {BreadcrumbItem} from "../types.ts";
import {useCurrentOrganization} from "../hooks/useCurrentOrganization.ts";
import {useStreams} from "../hooks/useStreams.ts";


export function Breadcrumbs() {
  const {projects, currentProject} = useProjects();
  const {currentStream} = useStreams();
  const currentOrganisation = useCurrentOrganization();
  const {navigate} = useRemails();

  if (!currentOrganisation) {
    return <></>
  }

  const items: BreadcrumbItem[] = [];
  if (projects) {
    items.push({title: 'Projects', route: 'projects'});
  }

  if (currentProject) {
    items.push({
      title: currentProject.name,
      route: 'projects.project',
    });
  }

  if (currentStream) {
    items.push({
      title: currentStream.name,
      route: 'projects.project.streams.stream',
    });
  }


  const anchors = items.map(item => (
    <Anchor key={item.route} onClick={() => navigate(item.route)}>
      {item.title}
    </Anchor>
  ));


  return (
    <MantineBreadcrumbs>{anchors}</MantineBreadcrumbs>
  )
}