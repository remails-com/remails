import { ActionDispatch, createContext, useContext, useReducer } from "react";
import { Action, State } from "../types.ts";
import { useRouter } from "./useRouter.ts";
import { routes } from "../routes.ts";
import { Navigate, Router } from "../router.ts";
import { reducer } from "../reducer.ts";
import apiMiddleware from "../apiMiddleware.ts";

export const RemailsContext = createContext<{ state: State; dispatch: ActionDispatch<[Action]>; navigate: Navigate }>({
  state: {
    user: null,
    organizations: null,
    projects: null,
    streams: null,
    messages: null,
    domains: null,
    organisationDomains: null,
    credentials: null,
    config: null,
    loading: false,
    routerState: {
      name: "",
      params: {},
    },
  },
  dispatch: () => {
    throw new Error("RemailsContext must be used within RemailsProvider");
  },
  navigate: () => {
    throw new Error("RemailsContext must be used within RemailsProvider");
  },
});

export function useRemails() {
  return useContext(RemailsContext);
}

const router = new Router(routes, window.location.pathname + window.location.search);

export function useLoadRemails() {
  const [state, dispatch] = useReducer(reducer, {
    user: null,
    organizations: null,
    projects: null,
    streams: null,
    messages: null,
    domains: null,
    organisationDomains: null,
    credentials: null,
    config: null,
    loading: false,
    routerState: router.initialState,
  });

  const navigate = useRouter(
    router,
    state,
    dispatch,
    [apiMiddleware]
  );

  return {
    state,
    dispatch,
    navigate,
  };
}
