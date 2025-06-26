import { ActionDispatch, createContext, useContext, useReducer } from "react";
import { Action, State, WhoamiResponse } from "../types.ts";
import { useRouter } from "./useRouter.ts";
import { routes } from "../routes.ts";
import { Navigate, Router } from "../router.ts";
import { useApi } from "./useApi.ts";
import { reducer } from "../reducer.ts";

export const RemailsContext = createContext<{ state: State; dispatch: ActionDispatch<[Action]>; navigate: Navigate }>({
  state: {
    organizations: null,
    projects: null,
    streams: null,
    messages: null,
    domains: null,
    credentials: null,
    config: null,
    loading: true,
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

export function useLoadRemails(user: WhoamiResponse | null) {
  const [state, dispatch] = useReducer(reducer, {
    organizations: null,
    projects: null,
    streams: null,
    messages: null,
    domains: null,
    credentials: null,
    config: null,
    loading: true,
    routerState: router.initialState,
  });

  const navigate = useRouter(router, dispatch);

  useApi(user, state, navigate, dispatch);

  return {
    state,
    dispatch,
    navigate,
  };
}
