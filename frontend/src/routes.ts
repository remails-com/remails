import { Route } from "./router";

export const routes: Route[] = [
  {
    name: "projects",
    path: "/{org_id}/projects",
    children: [
      {
        name: "project",
        path: "/{proj_id}",
        children: [
          {
            name: "domains",
            path: "/domains",
            children: [
              {
                name: "domain",
                path: "/{domain_id}",
              },
            ],
          },
          {
            name: "streams",
            path: "/streams",
            children: [
              {
                name: "stream",
                path: "/{stream_id}",
                children: [
                  {
                    name: "credentials",
                    path: "/credentials",
                    children: [
                      {
                        name: "credential",
                        path: "/{credential_id}",
                      },
                    ],
                  },
                  {
                    name: "messages",
                    path: "/messages",
                    children: [
                      {
                        name: "message",
                        path: "/{message_id}",
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
    children: [
      {
        name: "domain",
        path: "/{domain_id}",
      },
    ],
  },
  {
    name: "settings",
    path: "/{org_id}/settings",
  },
  {
    name: "statistics",
    path: "/{org_id}/statistics",
  },
  {
    name: "organizations",
    path: "/{org_id}/organizations",
  },
];
