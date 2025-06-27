import { ActionDispatch, createContext, useContext, useEffect, useReducer } from "react";
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

const router = new Router(routes);

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

  const navigate = useRouter(router, state, dispatch, [apiMiddleware]);

  useEffect(() => {
    // initial navigation
    const route = router.match(window.location.pathname + window.location.search);

    if (route) {
      navigate(route.name, route.params);
    } else {
      navigate("not_found");
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return {
    state,
    dispatch,
    navigate,
  };
}
