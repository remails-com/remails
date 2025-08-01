import { JSX } from "react";
import DomainDetails from "./components/domains/DomainDetails.tsx";
import DomainsOverview from "./components/domains/DomainsOverview.tsx";
import MessageDetails from "./components/messages/MessageDetails.tsx";
import OrganizationsOverview from "./components/organizations/OrganizationsOverview.tsx";
import ProjectDetails from "./components/projects/ProjectDetails.tsx";
import ProjectsOverview from "./components/projects/ProjectsOverview.tsx";
import OrganizationSettings from "./components/organizations/OrganizationSettings.tsx";
import { Setup } from "./components/Setup.tsx";
import CredentialDetails from "./components/smtpCredentials/CredentialDetails.tsx";
import StreamDetails from "./components/streams/StreamDetails.tsx";
import { useRemails } from "./hooks/useRemails.ts";
import { Dashboard } from "./layout/Dashboard";
import Login from "./Login.tsx";
import { RouteName } from "./routes.ts";
import UserSettings from "./components/userSettings/UserSettings.tsx";
import Statistics from "./components/statistics/Statistics.tsx";
import Error from "./error/Error.tsx";
import { ConfirmInvite } from "./components/ConfirmInvite.tsx";

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
  "projects.project.settings": <ProjectDetails />,
  domains: <DomainsOverview />,
  "domains.domain": <DomainDetails />,
  settings: <OrganizationSettings />,
  account: <UserSettings />,
  statistics: <Statistics />,
  organizations: <OrganizationsOverview />,
  default: null,
  login: null,
  invite: <ConfirmInvite />,
};

function Page() {
  const {
    state: { organizations, routerState },
  } = useRemails();

  if (organizations?.length === 0 && routerState.name != "invite") {
    return <Setup />;
  }

  return PageContent[routerState.name];
}

export function Pages() {
  const {
    state: { userFetched, routerState, error },
    dispatch,
  } = useRemails();

  if (error) {
    return <Error error={error} />;
  }

  if (!userFetched) {
    return null;
  }

  if (routerState.name === "login") {
    return <Login setUser={(user) => dispatch({ type: "set_user", user })} />;
  }

  return (
    <Dashboard>
      <Page />
    </Dashboard>
  );
}
