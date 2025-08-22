import { Dispatch } from "react";
import { NavigationState } from "./hooks/useRouter";
import { Action, Organization, WhoamiResponse } from "./types";
import { FullRouterState, RouteParams, Router } from "./router";
import { RemailsError } from "./error/error";

async function get<T>(path: string): Promise<T> {
  const response = await fetch(path, {
    method: "GET",
    headers: {
      Accept: "application/json",
    },
  });

  if (!response.ok) {
    throw new RemailsError(`Failed to fetch ${path} (${response.status} ${response.statusText})`, response.status);
  }

  return (await response.json()) as T;
}

export default async function apiMiddleware(
  navState: NavigationState,
  router: Router,
  dispatch: Dispatch<Action>
): Promise<FullRouterState> {
  let newOrgId = navState.to.params.org_id ?? null;
  let orgChanged = newOrgId !== navState.from.params.org_id && newOrgId !== null;

  let user: WhoamiResponse | null = navState.state.user;
  if (navState.state.user === null) {
    user = await get<WhoamiResponse>("/api/whoami");

    if (user === null || "error" in user) {
      dispatch({ type: "set_user", user: null });

      if (navState.to.name === "login") {
        return navState.to;
      } else {
        // If the user is not logged in, redirect to the login page
        const params: RouteParams = {};
        if (navState.to.name !== "default") {
          params.redirect = navState.to.fullPath;
        }

        return router.navigate("login", params);
      }
    }

    dispatch({ type: "set_user", user });
  }

  if (navState.state.config === null) {
    dispatch({ type: "set_config", config: await get("/api/config") });
  }

  let organizations = navState.state.organizations;
  if (navState.state.organizations === null) {
    organizations = await get<Organization[]>("/api/organizations");
    dispatch({ type: "set_organizations", organizations });
  }

  // navigate to the first organization if none is selected
  if (navState.to.name === "default" && user?.org_roles && organizations && organizations.length > 0) {
    newOrgId = user?.org_roles.find((r) => r.role === "admin")?.org_id || organizations[0].id;
    navState.to = router.navigate("projects", {
      org_id: newOrgId,
    });
    orgChanged = true;
  }

  const newProjId = navState.to.params.proj_id;
  const newStreamId = navState.to.params.stream_id;
  const projChanged = newProjId !== navState.from.params.proj_id && newProjId !== null;
  const streamChanged = newStreamId !== navState.from.params.stream_id && newStreamId !== null;

  if (orgChanged) {
    dispatch({ type: "set_projects", projects: await get(`/api/organizations/${newOrgId}/projects`) });
    dispatch({
      type: "set_domains",
      domains: await get(`/api/organizations/${newOrgId}/domains`),
      from_organization: true,
    });
  }

  if (projChanged && newProjId) {
    dispatch({
      type: "set_streams",
      streams: await get(`/api/organizations/${newOrgId}/projects/${newProjId}/streams`),
    });
    dispatch({
      type: "set_domains",
      domains: await get(`/api/organizations/${newOrgId}/projects/${newProjId}/domains`),
      from_organization: false,
    });
  }

  let messageFilterChanged = false;
  const messageFilter = new URLSearchParams();
  for (const param of ["limit", "status", "before"]) {
    const value = navState.to.params[param];
    if (value != navState.from.params[param]) messageFilterChanged = true;
    if (value) messageFilter.append(param, value);
  }
  if ((streamChanged || messageFilterChanged || navState.to.params.force == "reload") && newStreamId) {
    dispatch({
      type: "set_messages",
      messages: await get(
        `/api/organizations/${newOrgId}/projects/${newProjId}/streams/${newStreamId}/messages?${messageFilter.toString()}`
      ),
    });
  }

  if (streamChanged && newStreamId) {
    dispatch({
      type: "set_credentials",
      credentials: await get(
        `/api/organizations/${newOrgId}/projects/${newProjId}/streams/${newStreamId}/smtp_credentials`
      ),
    });
  }

  return navState.to;
}
