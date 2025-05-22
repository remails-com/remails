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

export function Pages() {
  const [opened, { open, close }] = useDisclosure(false);
  const {
    state: { route },
  } = useRemails();
  const { organizations } = useOrganizations();

  useEffect(() => {
    if (organizations?.length === 0) {
      open();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [organizations]);

  let element: ReactNode;

  switch (route.name) {
    case "organizations":
      element = <OrganizationsOverview />;
      break;
    case "projects":
      element = <ProjectsOverview />;
      break;
    case "project":
      element = <ProjectDetails />;
      break;
    case "stream":
      element = <StreamDetails />;
      break;
    case "domains":
      element = <DomainsOverview />;
      break;
    case "domain":
      element = <DomainDetails />;
      break;
    case "credential":
      element = <CredentialDetails />;
      break;
    case "settings":
      element = <Settings />;
      break;
    case "statistics":
      element = <Text>Organization wide statistics, quotas, etc.</Text>;
      break;
    case "message":
      element = <MessageDetails />;
      break;
    default:
      element = "Not Found";
  }

  return (
    <Dashboard>
      {organizations?.length === 0 && <Setup opened={opened} close={close} />}
      {element}
    </Dashboard>
  );
}
