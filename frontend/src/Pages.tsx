import { Dashboard } from "./layout/Dashboard";
import { OrganizationsOverview } from "./components/organizations/OrganizationsOverview";
import ProjectsOverview from "./components/projects/ProjectsOverview.tsx";
import { useRemails } from "./hooks/useRemails.ts";
import StreamDetails from "./components/streams/StreamDetails.tsx";
import ProjectDetails from "./components/projects/ProjectDetails.tsx";
import DomainsOverview from "./components/domains/DomainsOverview.tsx";
import { DomainDetails } from "./components/domains/DomainDetails.tsx";
import { CredentialDetails } from "./components/smtpCredentials/CredentialDetails.tsx";
import { Settings } from "./components/settings/Settings.tsx";
import { Setup } from "./components/Setup.tsx";
import MessageDetails from "./components/messages/MessageDetails.tsx";
import { NavigationProgress } from "@mantine/nprogress";
import { Quota } from "./components/statistics/Quota.tsx";
import { Login } from "./Login.tsx";
import { NotFound } from "./components/NotFound.tsx";

function Page() {
  const {
    state: { organizations, routerState },
  } = useRemails();
  const routeName = routerState.name;

  if (organizations?.length === 0) {
    return <Setup />;
  }

  if (routeName == "organizations") {
    return <OrganizationsOverview />;
  }

  if (routeName == "projects") {
    return <ProjectsOverview />;
  }

  if (routeName == "projects.project") {
    return <ProjectDetails />;
  }

  if (routeName == "projects.project.streams.stream") {
    return <StreamDetails />;
  }

  if (routeName == "domains") {
    return <DomainsOverview />;
  }

  if (routeName == "projects.project.domains") {
    return <ProjectDetails />;
  }

  if (routeName == "domains.domain") {
    return <DomainDetails />;
  }

  if (routeName == "projects.project.domains.domain") {
    return <DomainDetails />;
  }

  if (routeName == "projects.project.streams.stream.credentials.credential") {
    return <CredentialDetails />;
  }

  if (routeName == "settings") {
    return <Settings />;
  }

  if (routeName == "statistics") {
    return <Quota />;
  }

  if (routeName == "projects.project.streams.stream.messages.message") {
    return <MessageDetails />;
  }

  if (routeName == "default") {
    return null;
  }

  console.error("Unknown route:", routeName);
  return <NotFound />;
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
