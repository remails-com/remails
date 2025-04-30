import {ActionDispatch, createContext, useContext, useEffect, useReducer} from "react";
import {Action, State, WhoamiResponse} from "../types.ts";
import {initRouter, matchName, Navigate, RouteName, RouteParams, routerNavigate, routes} from "../router.ts";

function reducer(state: State, action: Action): State {
  console.log('fired action', action)

  if (action.type === 'set_organizations') {
    return {...state, organizations: action.organizations, loading: false}
  }

  if (action.type === 'loading') {
    return {...state, loading: true}
  }

  if (action.type === 'set_projects') {
    return {...state, projects: action.projects, loading: false}
  }

  if (action.type === 'add_project') {
    return {...state, projects: [...state.projects || [], action.project], loading: false}
  }

  if (action.type === 'remove_project') {
    return {...state, projects: state.projects?.filter(p => p.id !== action.projectId) || []}
  }

  if (action.type === 'set_streams') {
    return {...state, streams: action.streams, loading: false}
  }

  if (action.type === 'add_stream') {
    return {...state, streams: [...state.streams || [], action.stream], loading: false}
  }

  if (action.type === 'remove_stream') {
    return {...state, streams: state.streams?.filter(p => p.id !== action.streamId) || []}
  }

  if (action.type === 'set_messages') {
    return {...state, messages: action.messages, loading: false}
  }

  if (action.type === 'set_domains') {
    return {...state, domains: action.domains, loading: false}
  }

  if (action.type === 'add_domain') {
    return {...state, domains: [...state.domains || [], action.domain], loading: false}
  }

  if (action.type === 'remove_domain') {
    return {...state, domains: state.domains?.filter(d => d.id !== action.domainId) || []}
  }

  if (action.type === 'set_credentials') {
    return {...state, credentials: action.credentials, loading: false}
  }

  if (action.type === 'add_credential') {
    return {...state, credentials: [...state.credentials || [], action.credential], loading: false}
  }

  if (action.type === 'remove_credential') {
    return {...state, credentials: state.credentials?.filter(d => d.id !== action.credentialId) || []}
  }

  if (action.type === 'set_route') {
    return {
      ...state,
      route: action.route,
      fullPath: action.fullPath,
      fullName: action.fullName,
      pathParams: action.pathParams,
      queryParams: action.queryParams,
    }
  }

  console.error("unknown action", action)
  return state
}

export const RemailsContext = createContext<{ state: State, dispatch: ActionDispatch<[Action]>, navigate: Navigate }>(
  {
    state: {
      organizations: null,
      projects: null,
      streams: null,
      messages: null,
      domains: null,
      credentials: null,
      loading: true,
      route: routes[0],
      fullPath: "",
      fullName: "",
      queryParams: {},
      pathParams: {},
    },
    dispatch: () => {
      throw new Error('RemailsContext must be used within RemailsProvider');
    },
    navigate: (_name: RouteName, _params?: RouteParams) => {
      throw new Error("RemailsContext must be used within RemailsProvider");
    }
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
    loading: true,
    ...initialRoute
  });

  const navigate = (name: RouteName, pathParams: RouteParams = {}, queryParams = {}) => {
    dispatch({type: "set_route", ...routerNavigate(name, {...state.pathParams, ...pathParams}, queryParams)});
  };

  // handle back / forward events
  useEffect(() => {
    window.addEventListener('popstate', (event) => {
      if (event.state?.routeName) {
        dispatch({
          type: "set_route",
          pathParams: event.state?.routePathParams,
          queryParams: event.state?.routeQueryParams || {}, ...matchName(event.state.routeName)
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
  }, []);

  useEffect(() => {
    if (user) {
      fetch("/api/organizations")
        .then((res) => res.json())
        .then((data) => {
            if (Array.isArray(data)) {
              // TODO store this somehow, e.g., as cookie or in local storage
              dispatch({type: "set_organizations", organizations: data});
              if (!state.pathParams.org_id) {
                navigate('projects', {org_id: data[0].id});
              }
            }
          }
        )
    } else {
      dispatch({type: 'set_organizations', organizations: null})
    }

  }, [user]);

  // fetch projects when current organization changes
  useEffect(() => {
    const id = state.pathParams.org_id;

    if (id) {
      fetch(`/api/organizations/${id}/projects`)
        .then((res) => res.json())
        .then((data) => {
          if (Array.isArray(data)) {
            dispatch({type: 'set_projects', projects: data});
          }
        });
    } else {
      dispatch({type: 'set_projects', projects: null})
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
            dispatch({type: 'set_streams', streams: data});
          }
        });
    } else {
      dispatch({type: 'set_streams', streams: null})
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
            dispatch({type: 'set_messages', messages: data});
          }
        });
    } else {
      dispatch({type: 'set_messages', messages: null})
    }
  }, [user, state.pathParams.org_id, state.pathParams.proj_id, state.pathParams.stream_id]);

  useEffect(() => {
    const org_id = state.pathParams.org_id;
    const proj_id = state.pathParams.proj_id;

    let url: string;
    if (org_id && proj_id) {
      url = `/api/organizations/${org_id}/projects/${proj_id}/domains`
    } else if (org_id) {
      url = `/api/organizations/${org_id}/domains`
    } else {
      dispatch({type: 'set_domains', domains: null});
      return;
    }

    fetch(url)
      .then((res) => res.json())
      .then((data) => {
        if (Array.isArray(data)) {
          dispatch({type: 'set_domains', domains: data});
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
            dispatch({type: 'set_credentials', credentials: data});
          }
        });
    } else {
      dispatch({type: 'set_credentials', credentials: null});
      return;
    }
  }, [user, state.pathParams.org_id, state.pathParams.proj_id, state.pathParams.stream_id]);

  return {
    state,
    dispatch,
    navigate,
  };
}
