import {ReactNode} from 'react';
import {Dashboard} from './layout/Dashboard';
import {OrganizationsOverview} from './components/organizations/OrganizationsOverview';
import {ProjectsOverview} from "./components/projects/ProjectsOverview.tsx";
import {useRemails} from "./hooks/useRemails.ts";
import {StreamDetails} from "./components/streams/StreamDetails.tsx";
import {ProjectDetails} from "./components/projects/ProjectDetails.tsx";
import {DomainsOverview} from "./components/domains/DomainsOverview.tsx";
import {DomainDetails} from "./components/domains/DomainDetails.tsx";
import {CredentialDetails} from "./components/smtpCredentials/CredentialDetails.tsx";
import {Text} from "@mantine/core";

export function Pages() {
  const {state: {route}} = useRemails();

  let element: ReactNode;

  switch (route.name) {
    case 'organizations':
      element = <OrganizationsOverview/>
      break
    case 'projects':
      element = <ProjectsOverview/>
      break
    case 'project':
      element = <ProjectDetails/>
      break
    case 'stream':
      element = <StreamDetails/>
      break
    case 'domains':
      element = <DomainsOverview/>
      break
    case 'domain':
      element = <DomainDetails/>
      break
    case 'credential':
      element = <CredentialDetails/>
      break
    case 'settings':
      element = <Text>User account and Organization related settings (login, subscription, etc.)</Text>
      break
    case 'statistics':
      element = <Text>Organization wide statistics, quotas, etc.</Text>
      break
    default:
      element = "Not Found"
  }

  return (
    <Dashboard>
      {element}
    </Dashboard>
  );
}
