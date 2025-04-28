import {Anchor, Breadcrumbs as MantineBreadcrumbs} from "@mantine/core";
import {useProjects} from "../hooks/useProjects.ts";
import {useRemails} from "../hooks/useRemails.ts";
import {BreadcrumbItem} from "../types.ts";
import {useCurrentOrganization} from "../hooks/useCurrentOrganization.ts";
import {useStreams} from "../hooks/useStreams.ts";
import {useDomains} from "../hooks/useDomains.ts";


export function Breadcrumbs() {
  const {projects, currentProject} = useProjects();
  const {currentStream} = useStreams();
  const {domains, currentDomain} = useDomains();
  const currentOrganisation = useCurrentOrganization();
  const {navigate, state: {fullName}} = useRemails();

  if (!currentOrganisation) {
    return <></>
  }

  const items: BreadcrumbItem[] = [];
  if (projects && fullName.startsWith('projects')) {
    items.push({title: 'Projects', route: 'projects'});
  }

  if (domains && fullName.startsWith('domains')) {
    items.push({title: 'Domains', route: 'domains'});
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

  if (currentDomain) {
    let route = 'domains.domain'
    if (currentProject) {
      route = 'projects.project.domains.domain'
    }
    items.push({
      title: currentDomain.domain,
      route,
    })
  }


  const anchors = items.map(item => (
    <Anchor key={item.route} onClick={() => navigate(item.route)}>
      {item.title}
    </Anchor>
  ));


  return (
    <MantineBreadcrumbs mb="lg">{anchors}</MantineBreadcrumbs>
  )
}