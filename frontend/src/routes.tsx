import CredentialDetails from "./components/smtpCredentials/CredentialDetails.tsx";
import DomainDetails from "./components/domains/DomainDetails.tsx";
import DomainsOverview from "./components/domains/DomainsOverview.tsx";
import MessageDetails from "./components/messages/MessageDetails.tsx";
import NotFound from "./components/NotFound.tsx";
import OrganizationsOverview from "./components/organizations/OrganizationsOverview.tsx";
import ProjectDetails from "./components/projects/ProjectDetails.tsx";
import ProjectsOverview from "./components/projects/ProjectsOverview.tsx";
import Quota from "./components/statistics/Quota.tsx";
import Settings from "./components/settings/Settings.tsx";
import StreamDetails from "./components/streams/StreamDetails.tsx";
import { Route } from "./router.ts";

export const routes: Route[] = [
  {
    name: "projects",
    path: "/{org_id}/projects",
    content: <ProjectsOverview />,
    children: [
      {
        name: "project",
        path: "/{proj_id}",
        content: <ProjectDetails />,
        children: [
          {
            name: "domains",
            path: "/domains",
            content: null,
            children: [
              {
                name: "domain",
                path: "/{domain_id}",
                content: <DomainDetails />,
              },
            ],
          },
          {
            name: "streams",
            path: "/streams",
            content: null,
            children: [
              {
                name: "stream",
                path: "/{stream_id}",
                content: <StreamDetails />,
                children: [
                  {
                    name: "credentials",
                    path: "/credentials",
                    content: null,
                    children: [
                      {
                        name: "credential",
                        path: "/{credential_id}",
                        content: <CredentialDetails />,
                      },
                    ],
                  },
                  {
                    name: "messages",
                    path: "/messages",
                    content: null,
                    children: [
                      {
                        name: "message",
                        path: "/{message_id}",
                        content: <MessageDetails />,
                      },
                    ],
                  },
                ],
              },
            ],
          },
        ],
      },
    ],
  },
  {
    name: "domains",
    path: "/{org_id}/domains",
    content: <DomainsOverview />,
    children: [
      {
        name: "domain",
        path: "/{domain_id}",
        content: <DomainDetails />,
      },
    ],
  },
  {
    name: "settings",
    path: "/{org_id}/settings",
    content: <Settings />,
  },
  {
    name: "statistics",
    path: "/{org_id}/statistics",
    content: <Quota />,
  },
  {
    name: "organizations",
    path: "/{org_id}/organizations",
    content: <OrganizationsOverview />,
  },
  {
    name: "default",
    path: "/",
    content: null,
  },
  {
    name: "not_found",
    path: "/404",
    content: <NotFound />,
  },
  {
    name: "login",
    path: "/login",
    content: null,
  },
];
