import { Breadcrumbs as MantineBreadcrumbs, Text, Button, Box } from "@mantine/core";
import { useCredentials } from "../hooks/useCredentials.ts";
import { useDomains } from "../hooks/useDomains.ts";
import { useMessages } from "../hooks/useMessages.ts";
import { useOrganizations } from "../hooks/useOrganizations.ts";
import { useProjects } from "../hooks/useProjects.ts";
import { useRemails } from "../hooks/useRemails.ts";
import { RouteName } from "../routes.ts";
import { JSX } from "react";

interface SegmentProps {
  children: React.ReactNode;
  last?: boolean;
  route: RouteName;
}

function Segment({ children, last, route }: SegmentProps) {
  const { navigate } = useRemails();
  const props = { fz: "xs", c: "dark.3", px: "xs" };

  if (last) {
    return <Box {...props}>{children}</Box>;
  }

  return (
    <Button {...props} td="underline" size="xs" h={20} variant="transparent" onClick={() => navigate(route)}>
      {children}
    </Button>
  );
}

export function Breadcrumbs() {
  const { currentProject } = useProjects();
  const { currentCredential } = useCredentials();
  const { currentDomain } = useDomains();
  const { currentMessage } = useMessages();
  const { currentOrganization } = useOrganizations();
  const {
    state: { routerState },
  } = useRemails();

  if (!currentOrganization) {
    return null;
  }

  const items: JSX.Element[] = [];

  items.push(<Segment route="projects">{currentOrganization.name}</Segment>);

  const route_parts = routerState.name.split(".");

  for (let i = 0; i < route_parts.length; i++) {
    const route = route_parts.slice(0, i + 1).join(".") as RouteName;
    const isLast = i === route_parts.length - 1;

    let title: string | undefined = route_parts[i];

    // set user-defined name as breadcrumb title
    if (route == "projects.project") {
      title = currentProject?.name;
    } else if (route == "projects.project.emails.email") {
      let subject: string | null = null;
      if (currentMessage && "message_data" in currentMessage) {
        subject = currentMessage?.message_data?.subject;
      }
      title = subject ?? "No subject";
    } else if (route == "projects.project.credentials.credential") {
      title = currentCredential?.username;
    } else if (route == "domains.domain" || route == "projects.project.domains.domain") {
      title = currentDomain?.domain;
    }

    items.push(
      <Segment key={route} last={isLast} route={route}>
        {title}
      </Segment>
    );
  }

  return (
    <MantineBreadcrumbs
      mr="xl"
      separator={
        <Text component="span" size="sm" c="dimmed">
          /
        </Text>
      }
    >
      {items}
    </MantineBreadcrumbs>
  );
}
