import { expect, test } from "vitest";
import { matchName, matchPath, createRouteState, allRoutes } from "./router";

test("matchPath", () => {
  expect(matchPath(allRoutes, "/be90adce-695a-439b-84a2-62c8a0180f90/projects")).toStrictEqual({
    name: "projects",
    params: {
      org_id: "be90adce-695a-439b-84a2-62c8a0180f90",
    },
  });
  expect(
    matchPath(allRoutes, "/be90adce-695a-439b-84a2-62c8a0180f90/projects/9dc33958-00c0-4cf4-8219-9d522c076458")
  ).toStrictEqual({
    name: "projects.project",
    params: {
      org_id: "be90adce-695a-439b-84a2-62c8a0180f90",
      proj_id: "9dc33958-00c0-4cf4-8219-9d522c076458",
    },
  });
  expect(
    matchPath(
      allRoutes,
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
});

test("matchName", () => {
  expect(matchName(allRoutes, "projects")).toEqual("/{org_id}/projects");
  expect(matchName(allRoutes, "projects.project")).toEqual("/{org_id}/projects/{proj_id}");
  expect(matchName(allRoutes, "projects.project.streams.stream")).toEqual(
    "/{org_id}/projects/{proj_id}/streams/{stream_id}"
  );
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
