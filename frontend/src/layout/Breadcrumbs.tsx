import {Anchor, Breadcrumbs as MantineBreadcrumbs} from "@mantine/core";
import {useProjects} from "../hooks/useProjects.ts";
import {useRemails} from "../hooks/useRemails.ts";
import {BreadcrumbItem} from "../types.ts";
import {useCurrentOrganisation} from "../hooks/useCurrentOrganisation.ts";


export function Breadcrumbs() {
  const {projects, currentProject} = useProjects();
  const currentOrganisation = useCurrentOrganisation();
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
      params: {proj_id: currentProject.id, org_id: currentOrganisation.id}
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