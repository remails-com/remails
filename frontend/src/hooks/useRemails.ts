import {ActionDispatch, createContext, useContext, useEffect, useReducer} from "react";
import {Action, State} from "../types.ts";
import {_navigate, initRouter, matchName, Navigate, RouteName, RouteParams, routes} from "../router.ts";

function reducer(state: State, action: Action): State {
  console.log('fired action', action)
  if (action.type === 'load_organizations') {
    return {...state, organizations: [], currentOrganization: undefined, loading: true}
  }
  if (action.type === 'set_organizations') {
    return {...state, organizations: action.organizations, currentOrganization: action.organizations[0], loading: false}
  }
  if (action.type === 'set_current_organization') {
    return {
      ...state,
      ..._navigate('projects', {}, state),
      currentOrganization: action.organization,
      currentProject: undefined,
      projects: [],
      currentStream: undefined,
      streams: []
    }
  }
  if (action.type === 'load_projects') {
    return {...state, projects: [], currentProject: undefined, loading: true}
  }
  if (action.type === 'set_projects') {
    return {...state, projects: action.projects, loading: false}
  }
  if (action.type === 'set_current_project') {
    return {...state, currentProject: action.project}
  }
  if (action.type === 'load_streams') {
    return {...state, streams: [], currentStream: undefined, loading: true}
  }
  if (action.type === 'set_streams') {
    return {...state, streams: action.streams, currentStream: action.streams[0], loading: false}
  }
  if (action.type === 'set_current_stream') {
    return {...state, currentStream: action.stream}
  }
  if (action.type === 'set_route') {
    return {
      ...state,
      route: action.route,
      fullPath: action.fullPath,
      fullName: action.fullName,
      breadcrumbItems: action.breadcrumbItems,
      params: action.params,
    }
  }

  return state
}

export const RemailsContext = createContext<{ state: State, dispatch: ActionDispatch<[Action]>, navigate: Navigate }>(
  {
    state: {
      organizations: [],
      projects: [],
      streams: [],
      loading: true,
      route: routes[0],
      fullPath: "",
      fullName: "",
      params: {},
      breadcrumbItems: [],
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

export function useLoadRemails() {
  const initialRoute = initRouter();

  const [state, dispatch] = useReducer(reducer, {
    organizations: [],
    projects: [],
    streams: [],
    loading: true,
    ...initialRoute
  });

  // handle back / forward events
  useEffect(() => {
    window.addEventListener('popstate', (event) => {
      if (event.state?.routeName) {
        dispatch({type: "set_route", params: event.state?.routeParams || {}, ...matchName(event.state.routeName)});
      } else {
        dispatch({
          type: "set_route",
          params: initialRoute.params,
          route: initialRoute.route,
          fullPath: initialRoute.fullPath,
          fullName: initialRoute.fullName,
          breadcrumbItems: initialRoute.breadcrumbItems
        });
      }
    });
  }, []);

  useEffect(() => {
    fetch("/api/organizations")
      .then((res) => res.json())
      .then((data) => {
          if (Array.isArray(data)) {
            // TODO store this somehow, e.g., as cookie or in local storage
            dispatch({type: "set_organizations", organizations: data})
          }
        }
      )
  }, []);

  const navigate = (name: RouteName, params: RouteParams = {}) => {
    dispatch({type: "set_route", ..._navigate(name, params, state)});
  }

  return {
    state,
    dispatch,
    navigate,
  };
}