import {Anchor, Breadcrumbs as MantineBreadcrumbs} from "@mantine/core";
import {useProjects} from "../hooks/useProjects.ts";
import {useRemails} from "../hooks/useRemails.ts";
import {BreadcrumbItem} from "../types.ts";
import {useOrganizations} from "../hooks/useOrganizations.ts";
import {useStreams} from "../hooks/useStreams.ts";
import {useDomains} from "../hooks/useDomains.ts";
import {useCredentials} from "../hooks/useCredentials.ts";
import {useMessages} from "../hooks/useMessages.ts";


export function Breadcrumbs() {
  const {projects, currentProject} = useProjects();
  const {currentStream} = useStreams();
  const {currentCredential} = useCredentials();
  const {domains, currentDomain} = useDomains();
  const {currentMessage} = useMessages();
  const currentOrganisation = useOrganizations();
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

  if (currentCredential) {
    items.push({
      title: currentCredential.username,
      route: 'projects.project.streams.stream.credentials.credential',
    });
  }

  if (currentMessage && 'message_data' in currentMessage) {
    items.push({
      title: currentMessage.message_data.subject || 'No Subject',
      route: 'projects.project.streams.stream.message-log.message',
    });
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