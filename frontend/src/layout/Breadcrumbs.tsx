import { Anchor, Breadcrumbs as MantineBreadcrumbs } from "@mantine/core";
import { useProjects } from "../hooks/useProjects.ts";
import { useRemails } from "../hooks/useRemails.ts";
import { BreadcrumbItem } from "../types.ts";
import { useOrganizations } from "../hooks/useOrganizations.ts";
import { useStreams } from "../hooks/useStreams.ts";
import { useDomains } from "../hooks/useDomains.ts";
import { useCredentials } from "../hooks/useCredentials.ts";
import { useMessages } from "../hooks/useMessages.ts";

export function Breadcrumbs() {
  const { projects, currentProject } = useProjects();
  const { currentStream } = useStreams();
  const { currentCredential } = useCredentials();
  const { currentDomain } = useDomains();
  const { currentMessage } = useMessages();
  const { currentOrganization } = useOrganizations();
  const {
    navigate,
    state: { routerState },
  } = useRemails();

  if (!currentOrganization) {
    return null;
  }

  const items: BreadcrumbItem[] = [];

  items.push({ title: currentOrganization.name, route: "projects" });

  if (routerState.name === "settings") {
    items.push({ title: "Settings", route: "settings" });
  }

  if (routerState.name === "statistics") {
    items.push({ title: "Statistics", route: "statistics" });
  }

  if (projects && routerState.name.startsWith("projects")) {
    items.push({ title: "Projects", route: "projects" });
  }

  if (routerState.name.startsWith("domains")) {
    items.push({ title: "Domains", route: "domains" });
  }

  if (currentProject) {
    items.push({
      title: currentProject.name,
      route: "projects.project",
    });
  }

  if (currentStream) {
    items.push({
      title: "Streams",
      route: "projects.project",
      params: { tab: "Streams" },
    });
    items.push({
      title: currentStream.name,
      route: "projects.project.streams.stream",
    });
  }

  if (currentDomain) {
    const orgDomain = !routerState.params.proj_id;
    if (!orgDomain) {
      items.push({
        title: "Domains",
        route: "projects.project",
        params: { tab: "Domains" },
      });
    }
    items.push({
      title: currentDomain.domain,
      route: orgDomain ? "domains.domain" : "projects.project.domains.domain",
    });
  }

  if (currentCredential) {
    items.push({
      title: currentCredential.username,
      route: "projects.project.streams.stream.credentials.credential",
    });
  }

  if (currentMessage && "message_data" in currentMessage) {
    items.push({
      title: currentMessage.message_data.subject || "No Subject",
      route: "projects.project.streams.stream.messages.message",
    });
  }

  const anchors = items.map((item) => (
    <Anchor key={item.route} onClick={() => navigate(item.route, item.params)}>
      {item.title}
    </Anchor>
  ));

  return <MantineBreadcrumbs mb="lg">{anchors}</MantineBreadcrumbs>;
}
