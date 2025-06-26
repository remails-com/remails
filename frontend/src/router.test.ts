import { expect, test } from "vitest";
import { matchRoute, createRouteState, allRoutes, flattenRoutes } from "./router";

test("matchPath", () => {
  const flatRoutes = flattenRoutes(allRoutes);

  expect(matchRoute(flatRoutes, "/be90adce-695a-439b-84a2-62c8a0180f90/projects")).toStrictEqual({
    name: "projects",
    params: {
      org_id: "be90adce-695a-439b-84a2-62c8a0180f90",
    },
  });
  expect(
    matchRoute(flatRoutes, "/be90adce-695a-439b-84a2-62c8a0180f90/projects/9dc33958-00c0-4cf4-8219-9d522c076458")
  ).toStrictEqual({
    name: "projects.project",
    params: {
      org_id: "be90adce-695a-439b-84a2-62c8a0180f90",
      proj_id: "9dc33958-00c0-4cf4-8219-9d522c076458",
    },
  });
  expect(
    matchRoute(
      flatRoutes,
      "/be90adce-695a-439b-84a2-62c8a0180f90/projects/9dc33958-00c0-4cf4-8219-9d522c076458/streams/2969c252-9a47-4d81-ac4f-aef87ee42d28?tab=messages"
    )
  ).toStrictEqual({
    name: "projects.project.streams.stream",
    params: {
      org_id: "be90adce-695a-439b-84a2-62c8a0180f90",
      proj_id: "9dc33958-00c0-4cf4-8219-9d522c076458",
      stream_id: "2969c252-9a47-4d81-ac4f-aef87ee42d28",
      tab: "messages",
    },
  });
  // Trailing slashes
  expect(matchRoute(flatRoutes, "/be90adce-695a-439b-84a2-62c8a0180f90/projects/")).toStrictEqual({
    name: "projects",
    params: {
      org_id: "be90adce-695a-439b-84a2-62c8a0180f90",
    },
  });
  expect(
    matchRoute(
      flatRoutes,
      "/be90adce-695a-439b-84a2-62c8a0180f90/projects/9dc33958-00c0-4cf4-8219-9d522c076458/streams/2969c252-9a47-4d81-ac4f-aef87ee42d28/?tab=messages"
    )
  ).toStrictEqual({
    name: "projects.project.streams.stream",
    params: {
      org_id: "be90adce-695a-439b-84a2-62c8a0180f90",
      proj_id: "9dc33958-00c0-4cf4-8219-9d522c076458",
      stream_id: "2969c252-9a47-4d81-ac4f-aef87ee42d28",
      tab: "messages",
    },
  });
});

test("createRouteState", () => {
  expect(
    createRouteState("projects", {
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
    createRouteState("projects.project", {
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
    createRouteState("projects.project.streams.stream", {
      org_id: "be90adce-695a-439b-84a2-62c8a0180f90",
      proj_id: "9dc33958-00c0-4cf4-8219-9d522c076458",
      stream_id: "2969c252-9a47-4d81-ac4f-aef87ee42d28",
      tab: "messages",
    })
  ).toStrictEqual({
    name: "projects.project.streams.stream",
    fullPath:
      "/be90adce-695a-439b-84a2-62c8a0180f90/projects/9dc33958-00c0-4cf4-8219-9d522c076458/streams/2969c252-9a47-4d81-ac4f-aef87ee42d28?tab=messages",
    params: {
      org_id: "be90adce-695a-439b-84a2-62c8a0180f90",
      proj_id: "9dc33958-00c0-4cf4-8219-9d522c076458",
      stream_id: "2969c252-9a47-4d81-ac4f-aef87ee42d28",
      tab: "messages",
    },
  });
});

test("flattenRoutes", () => {
  expect(flattenRoutes(allRoutes)).toMatchObject([
    {
      name: "projects",
      path: "/{org_id}/projects",
    },
    {
      name: "projects.project",
      path: "/{org_id}/projects/{proj_id}",
    },
    {
      name: "projects.project.domains",
      path: "/{org_id}/projects/{proj_id}/domains",
    },
    {
      name: "projects.project.domains.domain",
      path: "/{org_id}/projects/{proj_id}/domains/{domain_id}",
    },
    {
      name: "projects.project.streams",
      path: "/{org_id}/projects/{proj_id}/streams",
    },
    {
      name: "projects.project.streams.stream",
      path: "/{org_id}/projects/{proj_id}/streams/{stream_id}",
    },
    {
      name: "projects.project.streams.stream.credentials",
      path: "/{org_id}/projects/{proj_id}/streams/{stream_id}/credentials",
    },
    {
      name: "projects.project.streams.stream.credentials.credential",
      path: "/{org_id}/projects/{proj_id}/streams/{stream_id}/credentials/{credential_id}",
    },
    {
      name: "projects.project.streams.stream.messages",
      path: "/{org_id}/projects/{proj_id}/streams/{stream_id}/messages",
    },
    {
      name: "projects.project.streams.stream.messages.message",
      path: "/{org_id}/projects/{proj_id}/streams/{stream_id}/messages/{message_id}",
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
      name: "statistics",
      path: "/{org_id}/statistics",
    },
    {
      name: "organizations",
      path: "/{org_id}/organizations",
    },
  ]);
});
