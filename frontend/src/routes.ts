import { Route } from "./router.ts";

export const routes = [
  {
    name: "projects",
    path: "/{org_id}/projects",
    children: [
      {
        name: "project",
        path: "/{proj_id}",
        children: [
          {
            name: "emails",
            path: "/emails",
            children: [
              {
                name: "email",
                path: "/{message_id}",
              },
            ],
          },
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
            name: "settings",
            path: "/settings",
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
        children: [
          {
            name: "settings",
            path: "/settings",
          },
        ],
      },
    ],
  },
  {
    name: "settings",
    path: "/{org_id}/settings",
    children: [
      {
        name: "members",
        path: "/members",
      },
      {
        name: "API keys",
        path: "/api_keys",
        children: [
          {
            name: "API key",
            path: "/{api_key_id}",
          },
        ],
      },
      {
        name: "admin",
        path: "/admin",
      },
    ],
  },
  {
    name: "account",
    path: "/{org_id}/account",
  },
  {
    name: "statistics",
    path: "/{org_id}/statistics",
  },
  {
    name: "organizations",
    path: "/{org_id}/organizations",
  },
  {
    name: "default",
    path: "/",
  },
  {
    name: "login",
    path: "/login",
  },
  {
    name: "mfa",
    path: "/mfa",
  },
  {
    name: "invite",
    path: "/invite/{new_org_id}/{invite_id}/{password}",
  },
] as const satisfies Route[];

// Recursive type to get all the route names from the Route[]
type GetRouteNames<R extends readonly Route[], Prefix extends string = ""> = {
  [K in keyof R]: R[K] extends { name: infer Name extends string; children?: readonly Route[] }
    ?
        | (Prefix extends "" ? Name : `${Prefix}.${Name}`)
        | (R[K]["children"] extends readonly Route[]
            ? GetRouteNames<R[K]["children"], Prefix extends "" ? Name : `${Prefix}.${Name}`>
            : never)
    : never;
}[number];

export type RouteName = GetRouteNames<typeof routes>;
