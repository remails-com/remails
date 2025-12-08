import { JSX } from "react";
import DomainsOverview from "./components/domains/DomainsOverview.tsx";
import MessageDetails from "./components/messages/MessageDetails.tsx";
import ProjectDetails from "./components/projects/ProjectDetails.tsx";
import ProjectsOverview from "./components/projects/ProjectsOverview.tsx";
import OrganizationSettings from "./components/organizations/OrganizationSettings.tsx";
import { Setup } from "./components/Setup.tsx";
import CredentialDetails from "./components/smtpCredentials/CredentialDetails.tsx";
import { useRemails } from "./hooks/useRemails.ts";
import { Dashboard } from "./layout/Dashboard";
import Login from "./Login.tsx";
import { RouteName } from "./routes.ts";
import UserSettings from "./components/userSettings/UserSettings.tsx";
import Statistics from "./components/statistics/Statistics.tsx";
import Error from "./error/Error.tsx";
import { ConfirmInvite } from "./components/ConfirmInvite.tsx";
import Mfa from "./Mfa.tsx";
import SetupSubscription from "./components/SetupSubscription.tsx";
import { useSubscription } from "./hooks/useSubscription.ts";
import ApiKeyDetails from "./components/apiKeys/ApiKeyDetails.tsx";
import DomainDetails from "./components/domains/DomainDetails.tsx";
import GlobalAdmin from "./components/admin/GlobalAdmin.tsx";
import PasswordReset from "./PasswordReset.tsx";

const PageContent: { [key in RouteName]: JSX.Element | null } = {
  projects: <ProjectsOverview />,
  "projects.project": <ProjectDetails />,
  "projects.project.emails": <ProjectDetails />,
  "projects.project.emails.email": <MessageDetails />,
  "projects.project.credentials": <ProjectDetails />,
  "projects.project.credentials.credential": <CredentialDetails />,
  "projects.project.settings": <ProjectDetails />,
  domains: <DomainsOverview />,
  "domains.domain": <DomainDetails />,
  "domains.domain.settings": <DomainDetails />,
  settings: <OrganizationSettings />,
  "settings.members": <OrganizationSettings />,
  "settings.API keys": <OrganizationSettings />,
  "settings.API keys.API key": <ApiKeyDetails />,
  "settings.admin": <OrganizationSettings />,
  admin: <GlobalAdmin />,
  "admin.organizations": <GlobalAdmin />,
  account: <UserSettings />,
  statistics: <Statistics />,
  default: null,
  login: null,
  "login.password_reset": null,
  mfa: null,
  invite: <ConfirmInvite />,
};

function Page() {
  const {
    state: { organizations, routerState },
  } = useRemails();
  const { subscription } = useSubscription();

  if (organizations?.length === 0 && routerState.name != "invite") {
    return <Setup />;
  }

  if (
    !(routerState.name === "settings") &&
    !routerState.name.startsWith("organizations") &&
    !routerState.name.startsWith("admin") &&
    subscription &&
    subscription.status !== "active"
  ) {
    return <SetupSubscription />;
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

  if (routerState.name === "login") {
    return <Login setUser={(user) => dispatch({ type: "set_user", user })} />;
  }

  if (routerState.name === "login.password_reset") {
    return <PasswordReset />;
  }

  if (routerState.name === "mfa") {
    return <Mfa setUser={(user) => dispatch({ type: "set_user", user })} />;
  }

  if (!userFetched) {
    return null;
  }

  return (
    <Dashboard>
      <Page />
    </Dashboard>
  );
}
