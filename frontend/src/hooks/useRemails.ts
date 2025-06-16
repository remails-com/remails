import { ActionDispatch, createContext, useCallback, useContext, useEffect, useReducer } from "react";
import { Action, State, WhoamiResponse } from "../types.ts";
import { initRouter, matchName, Navigate, RouteName, RouteParams, routerNavigate, routes } from "../router.ts";

const action_handler: {
  [action in Action["type"]]: (state: State, action: Extract<Action, { type: action }>) => State;
} = {
  set_organizations: function (state, action) {
    return { ...state, organizations: action.organizations, loading: false };
  },
  add_organization: function (state, action) {
    return { ...state, organizations: [...(state.organizations || []), action.organization], loading: false };
  },
  loading: function (state, _action) {
    return { ...state, loading: true };
  },
  set_projects: function (state, action) {
    return { ...state, projects: action.projects, loading: false };
  },
  add_project: function (state, action) {
    return { ...state, projects: [...(state.projects || []), action.project], loading: false };
  },
  remove_project: function (state, action) {
    return { ...state, projects: state.projects?.filter((p) => p.id !== action.projectId) || [] };
  },
  set_streams: function (state, action) {
    return { ...state, streams: action.streams, loading: false };
  },
  add_stream: function (state, action) {
    return { ...state, streams: [...(state.streams || []), action.stream], loading: false };
  },
  remove_stream: function (state, action) {
    return { ...state, streams: state.streams?.filter((p) => p.id !== action.streamId) || [] };
  },
  set_messages: function (state, action) {
    return { ...state, messages: action.messages, loading: false };
  },
  set_domains: function (state, action) {
    return { ...state, domains: action.domains, loading: false };
  },
  add_domain: function (state, action) {
    return { ...state, domains: [...(state.domains || []), action.domain], loading: false };
  },
  remove_domain: function (state, action) {
    return { ...state, domains: state.domains?.filter((d) => d.id !== action.domainId) || [] };
  },
  set_credentials: function (state, action) {
    return { ...state, credentials: action.credentials, loading: false };
  },
  add_credential: function (state, action) {
    return { ...state, credentials: [...(state.credentials || []), action.credential], loading: false };
  },
  remove_credential: function (state, action) {
    return { ...state, credentials: state.credentials?.filter((d) => d.id !== action.credentialId) || [] };
  },
  set_route: function (state, action) {
    return {
      ...state,
      route: action.route,
      fullPath: action.fullPath,
      fullName: action.fullName,
      pathParams: action.pathParams,
      queryParams: action.queryParams,
    };
  },
  set_config: function (state, action) {
    return { ...state, config: action.config };
  },
};

// helper function to make TypeScript recognize the proper types
function getActionHandler<T extends Action["type"]>(
  action: Extract<Action, { type: T }>
): (state: State, action: Extract<Action, { type: T }>) => State {
  return action_handler[action.type];
}

function reducer(state: State, action: Action): State {
  console.log("fired action", action);
  const handler = getActionHandler(action);
  return handler(state, action);
}

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
    route: routes[0],
    fullPath: "",
    fullName: "",
    queryParams: {},
    pathParams: {},
  },
  dispatch: () => {
    throw new Error("RemailsContext must be used within RemailsProvider");
  },
  navigate: (_name: RouteName, _params?: RouteParams) => {
    throw new Error("RemailsContext must be used within RemailsProvider");
  },
});

export function useRemails() {
  return useContext(RemailsContext);
}

export function useLoadRemails(user: WhoamiResponse | null) {
  const initialRoute = initRouter();

  const [state, dispatch] = useReducer(reducer, {
    organizations: null,
    projects: null,
    streams: null,
    messages: null,
    domains: null,
    credentials: null,
    config: null,
    loading: true,
    ...initialRoute,
  });

  const navigate = useCallback(
    (name: RouteName, pathParams: RouteParams = {}, queryParams = {}) => {
      dispatch({ type: "set_route", ...routerNavigate(name, { ...state.pathParams, ...pathParams }, queryParams) });
    },
    [state.pathParams]
  );

  // handle back / forward events
  useEffect(() => {
    window.addEventListener("popstate", (event) => {
      if (event.state?.routeName) {
        dispatch({
          type: "set_route",
          pathParams: event.state?.routePathParams,
          queryParams: event.state?.routeQueryParams || {},
          ...matchName(event.state.routeName),
        });
      } else {
        dispatch({
          type: "set_route",
          pathParams: initialRoute.pathParams,
          queryParams: initialRoute.queryParams,
          route: initialRoute.route,
          fullPath: initialRoute.fullPath,
          fullName: initialRoute.fullName,
        });
      }
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (user) {
      fetch("/api/organizations")
        .then((res) => res.json())
        .then((data) => {
          if (Array.isArray(data)) {
            // TODO store this somehow, e.g., as cookie or in local storage
            dispatch({ type: "set_organizations", organizations: data });
            if (!state.pathParams.org_id && data.length > 0) {
              navigate("projects", { org_id: data[0].id });
            }
          }
        });
    } else {
      dispatch({ type: "set_organizations", organizations: null });
    }
  }, [user, navigate, state.pathParams.org_id]);

  // fetch projects when current organization changes
  useEffect(() => {
    const id = state.pathParams.org_id;

    if (id) {
      fetch(`/api/organizations/${id}/projects`)
        .then((res) => res.json())
        .then((data) => {
          if (Array.isArray(data)) {
            dispatch({ type: "set_projects", projects: data });
          }
        });
    } else {
      dispatch({ type: "set_projects", projects: null });
    }
  }, [user, state.pathParams.org_id]);

  // fetch streams when current project changes
  useEffect(() => {
    const org_id = state.pathParams.org_id;
    const proj_id = state.pathParams.proj_id;

    if (org_id && proj_id) {
      fetch(`/api/organizations/${org_id}/projects/${proj_id}/streams`)
        .then((res) => res.json())
        .then((data) => {
          if (Array.isArray(data)) {
            dispatch({ type: "set_streams", streams: data });
          }
        });
    } else {
      dispatch({ type: "set_streams", streams: null });
    }
  }, [user, state.pathParams.org_id, state.pathParams.proj_id]);

  useEffect(() => {
    const org_id = state.pathParams.org_id;
    const proj_id = state.pathParams.proj_id;
    const stream_id = state.pathParams.stream_id;

    if (org_id && proj_id && stream_id) {
      fetch(`/api/organizations/${org_id}/projects/${proj_id}/streams/${stream_id}/messages`)
        .then((res) => res.json())
        .then((data) => {
          if (Array.isArray(data)) {
            dispatch({ type: "set_messages", messages: data });
          }
        });
    } else {
      dispatch({ type: "set_messages", messages: null });
    }
  }, [user, state.pathParams.org_id, state.pathParams.proj_id, state.pathParams.stream_id]);

  useEffect(() => {
    const org_id = state.pathParams.org_id;
    const proj_id = state.pathParams.proj_id;

    let url: string;
    if (org_id && proj_id) {
      url = `/api/organizations/${org_id}/projects/${proj_id}/domains`;
    } else if (org_id) {
      url = `/api/organizations/${org_id}/domains`;
    } else {
      dispatch({ type: "set_domains", domains: null });
      return;
    }

    fetch(url)
      .then((res) => res.json())
      .then((data) => {
        if (Array.isArray(data)) {
          dispatch({ type: "set_domains", domains: data });
        }
      });
  }, [user, state.pathParams.org_id, state.pathParams.proj_id]);

  useEffect(() => {
    const org_id = state.pathParams.org_id;
    const proj_id = state.pathParams.proj_id;
    const stream_id = state.pathParams.stream_id;

    if (org_id && proj_id && stream_id) {
      fetch(`/api/organizations/${org_id}/projects/${proj_id}/streams/${stream_id}/smtp_credentials`)
        .then((res) => res.json())
        .then((data) => {
          if (Array.isArray(data)) {
            dispatch({ type: "set_credentials", credentials: data });
          }
        });
    } else {
      dispatch({ type: "set_credentials", credentials: null });
      return;
    }
  }, [user, state.pathParams.org_id, state.pathParams.proj_id, state.pathParams.stream_id]);

  useEffect(() => {
    fetch("/api/config")
      .then((res) => res.json())
      .then((data) => dispatch({ type: "set_config", config: data }));
  }, []);

  return {
    state,
    dispatch,
    navigate,
  };
}
