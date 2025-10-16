import { ActionDispatch, createContext, useContext, useEffect, useReducer } from "react";
import { Action, State } from "../types.ts";
import { useRouter } from "./useRouter.ts";
import { routes } from "../routes.ts";
import { Navigate, Router } from "../router.ts";
import { reducer } from "../reducer.ts";
import apiMiddleware from "../apiMiddleware.ts";
import { RemailsError } from "../error/error.ts";

export const RemailsContext = createContext<{
  state: State;
  dispatch: ActionDispatch<[Action]>;
  navigate: Navigate;
  // Redirect to the page in the `redirect` query param. Used for navigating to the right page after loging in.
  redirect: () => void;
  match: Router["match"];
}>({
  state: {
    user: null,
    userFetched: false,
    totpCodes: null,
    organizations: null,
    projects: null,
    streams: null,
    messages: null,
    domains: null,
    organizationDomains: null,
    credentials: null,
    apiKeys: null,
    config: null,
    routerState: {
      name: "default",
      params: {},
    },
    nextRouterState: null,
    error: null,
  },
  dispatch: () => {
    throw new Error("RemailsContext must be used within RemailsProvider");
  },
  navigate: () => {
    throw new Error("RemailsContext must be used within RemailsProvider");
  },
  match: () => {
    throw new Error("RemailsContext must be used within RemailsProvider");
  },
  redirect: () => {
    throw new Error("RemailsContext must be used within RemailsProvider");
  },
});

export function useRemails() {
  return useContext(RemailsContext);
}

const router = new Router(routes);

export function useLoadRemails() {
  const [state, dispatch] = useReducer(reducer, {
    user: null,
    userFetched: false,
    totpCodes: null,
    organizations: null,
    projects: null,
    streams: null,
    messages: null,
    domains: null,
    organizationDomains: null,
    credentials: null,
    error: null,
    config: null,
    routerState: router.initialState,
    nextRouterState: null,
    apiKeys: null,
  });

  const { navigate, redirect } = useRouter(router, state, dispatch, [apiMiddleware]);

  useEffect(() => {
    // initial navigation
    const route = router.match(window.location.pathname + window.location.search);

    if (route) {
      navigate(route.name, route.params);
    } else {
      throw new RemailsError(`Route ${window.location.pathname} doesn't exist`, 404);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return {
    state,
    dispatch,
    navigate,
    redirect,
    match: router.match.bind(router),
  };
}
