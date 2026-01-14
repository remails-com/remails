import { expect, test } from "vitest";
import { flattenRoutes, Route, Router } from "./router";

// Constant routes for router unit tests
const routes = [
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
                path: "/{email_id}",
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
] as const satisfies Route[];

test("Match a path", () => {
  const router = new Router(routes);

  expect(router.match("/be90adce-695a-439b-84a2-62c8a0180f90/projects")).toStrictEqual({
    name: "projects",
    params: {
      org_id: "be90adce-695a-439b-84a2-62c8a0180f90",
    },
  });
  expect(
    router.match("/be90adce-695a-439b-84a2-62c8a0180f90/projects/9dc33958-00c0-4cf4-8219-9d522c076458")
  ).toStrictEqual({
    name: "projects.project",
    params: {
      org_id: "be90adce-695a-439b-84a2-62c8a0180f90",
      proj_id: "9dc33958-00c0-4cf4-8219-9d522c076458",
    },
  });
  expect(
    router.match("/be90adce-695a-439b-84a2-62c8a0180f90/projects/9dc33958-00c0-4cf4-8219-9d522c076458?tab=emails")
  ).toStrictEqual({
    name: "projects.project",
    params: {
      org_id: "be90adce-695a-439b-84a2-62c8a0180f90",
      proj_id: "9dc33958-00c0-4cf4-8219-9d522c076458",
      tab: "emails",
    },
  });
  // Trailing slashes
  expect(router.match("/be90adce-695a-439b-84a2-62c8a0180f90/projects/")).toStrictEqual({
    name: "projects",
    params: {
      org_id: "be90adce-695a-439b-84a2-62c8a0180f90",
    },
  });
  expect(
    router.match("/be90adce-695a-439b-84a2-62c8a0180f90/projects/9dc33958-00c0-4cf4-8219-9d522c076458/?tab=emails")
  ).toStrictEqual({
    name: "projects.project",
    params: {
      org_id: "be90adce-695a-439b-84a2-62c8a0180f90",
      proj_id: "9dc33958-00c0-4cf4-8219-9d522c076458",
      tab: "emails",
    },
  });
});

test("createRouteState", () => {
  const router = new Router(routes);

  expect(
    router.navigate("projects", {
      org_id: "be90adce-695a-439b-84a2-62c8a0180f90",
    })
  ).toStrictEqual({
    name: "projects",
    fullPath: "/be90adce-695a-439b-84a2-62c8a0180f90/projects",
    params: {
      org_id: "be90adce-695a-439b-84a2-62c8a0180f90",
    },
  });
  expect(
    router.navigate("projects.project", {
      org_id: "be90adce-695a-439b-84a2-62c8a0180f90",
      proj_id: "9dc33958-00c0-4cf4-8219-9d522c076458",
    })
  ).toStrictEqual({
    name: "projects.project",
    fullPath: "/be90adce-695a-439b-84a2-62c8a0180f90/projects/9dc33958-00c0-4cf4-8219-9d522c076458",
    params: {
      org_id: "be90adce-695a-439b-84a2-62c8a0180f90",
      proj_id: "9dc33958-00c0-4cf4-8219-9d522c076458",
    },
  });
  expect(
    router.navigate("projects.project", {
      org_id: "be90adce-695a-439b-84a2-62c8a0180f90",
      proj_id: "9dc33958-00c0-4cf4-8219-9d522c076458",
      tab: "emails",
    })
  ).toStrictEqual({
    name: "projects.project",
    fullPath: "/be90adce-695a-439b-84a2-62c8a0180f90/projects/9dc33958-00c0-4cf4-8219-9d522c076458?tab=emails",
    params: {
      org_id: "be90adce-695a-439b-84a2-62c8a0180f90",
      proj_id: "9dc33958-00c0-4cf4-8219-9d522c076458",
      tab: "emails",
    },
  });
});

test("flattenRoutes", () => {
  expect(flattenRoutes(routes)).toMatchObject([
    {
      name: "projects",
      path: "/{org_id}/projects",
    },
    {
      name: "projects.project",
      path: "/{org_id}/projects/{proj_id}",
    },
    {
      name: "projects.project.emails",
      path: "/{org_id}/projects/{proj_id}/emails",
    },
    {
      name: "projects.project.emails.email",
      path: "/{org_id}/projects/{proj_id}/emails/{email_id}",
    },
    {
      name: "projects.project.credentials",
      path: "/{org_id}/projects/{proj_id}/credentials",
    },
    {
      name: "projects.project.credentials.credential",
      path: "/{org_id}/projects/{proj_id}/credentials/{credential_id}",
    },
    {
      name: "projects.project.settings",
      path: "/{org_id}/projects/{proj_id}/settings",
    },
    {
      name: "domains",
      path: "/{org_id}/domains",
    },
    {
      name: "domains.domain",
      path: "/{org_id}/domains/{domain_id}",
    },
    {
      name: "settings",
      path: "/{org_id}/settings",
    },
    {
      name: "settings.members",
      path: "/{org_id}/settings/members",
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
  ]);
});
