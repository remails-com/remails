import { Anchor, Breadcrumbs as MantineBreadcrumbs, Text } from "@mantine/core";
import { useCredentials } from "../hooks/useCredentials.ts";
import { useDomains } from "../hooks/useDomains.ts";
import { useMessages } from "../hooks/useMessages.ts";
import { useOrganizations } from "../hooks/useOrganizations.ts";
import { useProjects } from "../hooks/useProjects.ts";
import { useRemails } from "../hooks/useRemails.ts";
import { useStreams } from "../hooks/useStreams.ts";
import { RouteName } from "../routes.tsx";
import { JSX } from "react";

interface BreadcrumbItem {
  title: string | JSX.Element;
  route: RouteName;
}

export function Breadcrumbs() {
  const { currentProject } = useProjects();
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

  items.push({ title: <Text fs="italic">{currentOrganization.name}</Text>, route: "projects" });

  const route_parts = routerState.name.split(".");

  for (let i = 0; i < route_parts.length; i++) {
    const route = route_parts.slice(0, i + 1).join(".") as RouteName;

    let title: string | JSX.Element = route_parts[i];

    // set user-defined name as breadcrumb title
    if (route == "projects.project") {
      title = <Text fs="italic">{currentProject?.name}</Text>;
    }
    if (route == "projects.project.streams.stream") {
      title = <Text fs="italic">{currentStream?.name}</Text>;
    }
    if (route == "projects.project.streams.stream.messages.message") {
      let subject: string | null = null;
      if (currentMessage && "message_data" in currentMessage) {
        subject = currentMessage?.message_data?.subject;
      }
      title = <Text fs="italic">{subject ?? "no subject"}</Text>;
    }
    if (route == "projects.project.streams.stream.credentials.credential") {
      title = <Text fs="italic">{currentCredential?.username}</Text>;
    }
    if (route == "domains.domain" || route == "projects.project.domains.domain") {
      title = <Text fs="italic">{currentDomain?.domain}</Text>;
    }

    items.push({ title, route });
  }

  const anchors = items.map((item) => (
    <Anchor key={item.route} onClick={() => navigate(item.route)}>
      {item.title}
    </Anchor>
  ));

  return <MantineBreadcrumbs mb="lg">{anchors}</MantineBreadcrumbs>;
}
