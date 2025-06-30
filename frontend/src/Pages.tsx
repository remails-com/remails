import { ReactNode, useEffect } from "react";
import { Dashboard } from "./layout/Dashboard";
import { OrganizationsOverview } from "./components/organizations/OrganizationsOverview";
import ProjectsOverview from "./components/projects/ProjectsOverview.tsx";
import { useRemails } from "./hooks/useRemails.ts";
import StreamDetails from "./components/streams/StreamDetails.tsx";
import ProjectDetails from "./components/projects/ProjectDetails.tsx";
import DomainsOverview from "./components/domains/DomainsOverview.tsx";
import { DomainDetails } from "./components/domains/DomainDetails.tsx";
import { CredentialDetails } from "./components/smtpCredentials/CredentialDetails.tsx";
import { Text } from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import { useOrganizations } from "./hooks/useOrganizations.ts";
import { Settings } from "./components/settings/Settings.tsx";
import { Setup } from "./components/Setup.tsx";
import MessageDetails from "./components/messages/MessageDetails.tsx";
import { nprogress, NavigationProgress } from "@mantine/nprogress";

export function Pages() {
  const [opened, { open, close }] = useDisclosure(false);
  const {
    state: { routerState, loading },
  } = useRemails();
  const { organizations } = useOrganizations();

  useEffect(() => {
    if (organizations?.length === 0) {
      open();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [organizations]);

  useEffect(() => {
    if (loading) {
      nprogress.start();
    } else {
      nprogress.complete();
    }
  }, [loading]);

  let element: ReactNode;

  switch (routerState.name) {
    case "organizations":
      element = <OrganizationsOverview />;
      break;
    case "projects":
      element = <ProjectsOverview />;
      break;
    case "projects.project":
      element = <ProjectDetails />;
      break;
    case "projects.project.streams.stream":
      element = <StreamDetails />;
      break;
    case "domains":
    case "projects.project.domains":
      element = <DomainsOverview />;
      break;
    case "domains.domain":
    case "projects.project.domains.domain":
      element = <DomainDetails />;
      break;
    case "projects.project.streams.stream.credentials.credential":
      element = <CredentialDetails />;
      break;
    case "settings":
      element = <Settings />;
      break;
    case "statistics":
      element = <Text>Organization wide statistics, quotas, etc.</Text>;
      break;
    case "projects.project.streams.stream.messages.message":
      element = <MessageDetails />;
      break;
    case "not_found":
      element = <NavigationProgress />;
      break;
    default:
      console.error("Unknown route:", routerState.name);
      element = "Not Found";
  }

  return (
    <Dashboard>
      <NavigationProgress />
      {organizations?.length === 0 && <Setup opened={opened} close={close} />}
      {element}
    </Dashboard>
  );
}
