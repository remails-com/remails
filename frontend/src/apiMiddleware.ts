import { Dispatch } from "react";
import { NavigationState } from "./hooks/useRouter";
import { Action, Organization, User, WhoamiResponse } from "./types";
import { FullRouterState, RouteParams, Router } from "./router";
import { RemailsError } from "./error/error";

export async function get<T>(path: string): Promise<T> {
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

  let user: User;
  if (navState.state.user === null || navState.to.params.force == "reload-orgs") {
    const updated_user = await get<WhoamiResponse>("/api/whoami");

    if (updated_user === null || "error" in updated_user) {
      dispatch({ type: "set_user", user: null });

      if (navState.to.name.startsWith("login")) {
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

    if (updated_user.login_status === "mfa_pending") {
      // If the user has to finish MFA, redirect to the MFA page
      const params: RouteParams = navState.state.routerState.params;
      if (navState.to.name !== "mfa") {
        params.redirect = navState.to.fullPath;
      }

      return router.navigate("mfa", params);
    }

    dispatch({ type: "set_user", user: updated_user });
    user = updated_user;
  } else {
    user = navState.state.user;
  }

  if (!user) {
    throw new RemailsError("Could not log in", 401);
  }

  if (navState.state.config === null) {
    dispatch({ type: "set_config", config: await get("/api/config") });
  }

  if (user.global_role === "admin" && navState.state.runtimeConfig === null) {
    dispatch({ type: "set_runtime_config", config: await get("/api/config/runtime") });
    dispatch({ type: "set_api_users", users: await get("/api/api_user") });
  }

  if (navState.state.totpCodes === null) {
    dispatch({ type: "set_totp_codes", totpCodes: await get(`/api/api_user/${user.id}/totp`) });
  }

  let organizations = navState.state.organizations;
  if (navState.state.organizations === null || navState.to.params.force == "reload-orgs") {
    organizations = await get<Organization[]>("/api/organizations");
    dispatch({ type: "set_organizations", organizations });
  }

  // navigate to the first organization if none is selected
  if (navState.to.name === "default" && user.org_roles && organizations && organizations.length > 0) {
    newOrgId = user.org_roles.find((r) => r.role === "admin")?.org_id || organizations[0].id;
    navState.to = router.navigate("projects", {
      org_id: newOrgId,
    });
    orgChanged = true;
  }

  const newProjId = navState.to.params.proj_id;
  const projChanged = newProjId !== navState.from.params.proj_id && newProjId !== null;

  if (orgChanged) {
    dispatch({ type: "set_projects", projects: await get(`/api/organizations/${newOrgId}/projects`) });
    dispatch({
      type: "set_domains",
      domains: await get(`/api/organizations/${newOrgId}/domains`),
    });
  }

  if (navState.to.name == "statistics") {
    dispatch({ type: "set_statistics", statistics: await get(`/api/organizations/${newOrgId}/statistics`) });
  }

  if (projChanged && newProjId) {
    dispatch({
      type: "set_credentials",
      credentials: await get(`/api/organizations/${newOrgId}/projects/${newProjId}/smtp_credentials`),
    });
    dispatch({
      type: "set_labels",
      labels: await get(`/api/organizations/${newOrgId}/projects/${newProjId}/labels`),
    });
  }

  let messageFilterChanged = false;
  const messageFilter = new URLSearchParams();
  for (const param of ["limit", "status", "before", "labels"]) {
    const value = navState.to.params[param];
    if (value != navState.from.params[param]) messageFilterChanged = true;
    if (value) messageFilter.append(param, value);
  }
  if ((projChanged || messageFilterChanged || navState.to.params.force == "reload") && newProjId) {
    dispatch({
      type: "set_messages",
      messages: await get(`/api/organizations/${newOrgId}/projects/${newProjId}/emails?${messageFilter.toString()}`),
    });
  }

  if (
    navState.to.name == "settings.API keys" ||
    (!navState.state.apiKeys && navState.to.name.startsWith("settings.API keys"))
  ) {
    dispatch({
      type: "set_api_keys",
      apiKeys: await get(`/api/organizations/${newOrgId}/api_keys`),
    });
  }

  return navState.to;
}
