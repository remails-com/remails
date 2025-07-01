import { Dispatch } from "react";
import { NavigationState } from "./hooks/useRouter";
import { Action, Organization, User, WhoamiResponse } from "./types";
import { FullRouterState, Router } from "./router";

async function get<T>(path: string): Promise<T> {
  const response = await fetch(path, {
    method: "GET",
    headers: {
      Accept: "application/json",
    },
  });

  if (!response.ok) {
    throw new Error(`Failed to fetch ${path}: ${response.status} ${response.statusText}`);
  }

  return response.json() as T;
}

export default async function apiMiddleware(
  navState: NavigationState,
  router: Router,
  dispatch: Dispatch<Action>
): Promise<FullRouterState> {
  let newOrgId = navState.to.params.org_id;
  let orgChanged = newOrgId !== navState.from.params.org_id && newOrgId !== null;

  if (navState.state.user === null) {
    const user = await get<WhoamiResponse>("/api/whoami");

    if (user === null || "error" in user) {
      dispatch({ type: "set_user", user: null });
      // If the user is not logged in, redirect to the login page
      return router.navigate("default", {});
    }

    dispatch({ type: "set_user", user: user as User });
  }

  if (navState.state.config === null) {
    dispatch({ type: "set_config", config: await get("/api/config") });
  }

  if (navState.state.organizations === null) {
    const organizations = await get<Organization[]>("/api/organizations");
    dispatch({ type: "set_organizations", organizations });

    // navigate to the first organization if none is selected
    if (!navState.to.params.org_id && organizations.length > 0) {
      navState.to = router.navigate("projects", { org_id: organizations[0].id });
      orgChanged = true;
      newOrgId = organizations[0].id;
    }
  }

  const newProjId = navState.to.params.proj_id;
  const newStreamId = navState.to.params.stream_id;
  const projChanged = newProjId !== navState.from.params.proj_id && newProjId !== null;
  const streamChanged = newStreamId !== navState.from.params.stream_id && newStreamId !== null;

  if (orgChanged) {
    dispatch({ type: "set_projects", projects: await get(`/api/organizations/${newOrgId}/projects`) });
    dispatch({
      type: "set_organization_domains",
      organizationDomains: await get(`/api/organizations/${newOrgId}/domains`),
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
