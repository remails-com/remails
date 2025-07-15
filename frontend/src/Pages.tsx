import { NavigationProgress } from "@mantine/nprogress";
import { JSX } from "react";
import DomainDetails from "./components/domains/DomainDetails.tsx";
import DomainsOverview from "./components/domains/DomainsOverview.tsx";
import MessageDetails from "./components/messages/MessageDetails.tsx";
import NotFound from "./components/NotFound.tsx";
import OrganizationsOverview from "./components/organizations/OrganizationsOverview.tsx";
import ProjectDetails from "./components/projects/ProjectDetails.tsx";
import ProjectsOverview from "./components/projects/ProjectsOverview.tsx";
import Settings from "./components/settings/Settings.tsx";
import { Setup } from "./components/Setup.tsx";
import CredentialDetails from "./components/smtpCredentials/CredentialDetails.tsx";
import Quota from "./components/statistics/Quota.tsx";
import StreamDetails from "./components/streams/StreamDetails.tsx";
import { useRemails } from "./hooks/useRemails.ts";
import { Dashboard } from "./layout/Dashboard";
import Login from "./Login.tsx";
import { RouteName } from "./routes.ts";

const PageContent: { [key in RouteName]: JSX.Element | null } = {
  projects: <ProjectsOverview />,
  "projects.project": <ProjectDetails />,
  "projects.project.streams": <ProjectDetails />,
  "projects.project.streams.stream": <StreamDetails />,
  "projects.project.streams.stream.messages": <StreamDetails />,
  "projects.project.streams.stream.messages.message": <MessageDetails />,
  "projects.project.streams.stream.credentials": <StreamDetails />,
  "projects.project.streams.stream.credentials.credential": <CredentialDetails />,
  "projects.project.streams.stream.settings": <StreamDetails />,
  "projects.project.domains": <ProjectDetails />,
  "projects.project.domains.domain": <DomainDetails />,
  "projects.project.domains.domain.dns": <DomainDetails />,
  "projects.project.settings": <ProjectDetails />,
  domains: <DomainsOverview />,
  "domains.domain": <DomainDetails />,
  "domains.domain.dns": <DomainDetails />,
  settings: <Settings />,
  statistics: <Quota />,
  organizations: <OrganizationsOverview />,
  default: null,
  not_found: <NotFound />,
  login: null,
};

function Page() {
  const {
    state: { organizations, routerState },
  } = useRemails();
  if (organizations?.length === 0) {
    return <Setup />;
  }

  return PageContent[routerState.name];
}

export function Pages() {
  const {
    state: {
      userFetched,
      routerState: { name },
    },
    dispatch,
  } = useRemails();

  if (!userFetched) {
    return <NavigationProgress />;
  }

  if (name === "login") {
    return <Login setUser={(user) => dispatch({ type: "set_user", user })} />;
  }

  if (name === "not_found") {
    return <NotFound />;
  }

  return (
    <Dashboard>
      <NavigationProgress />
      <Page />
    </Dashboard>
  );
}
