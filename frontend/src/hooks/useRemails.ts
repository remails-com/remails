import {ActionDispatch, createContext, useContext, useEffect, useReducer} from "react";
import {Action, State} from "../types.ts";

function reducer(state: State, action: Action): State {
  if (action.type === 'load_organizations') {
    return {...state, organizations: [], currentOrganization: undefined, loading: true}
  }
  if (action.type === 'set_organizations') {
    return {...state, organizations: action.organizations, currentOrganization: action.organizations[0], loading: false}
  }
  if (action.type === 'set_current_organization') {
    return {
      ...state,
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
    return {...state, projects: action.projects, currentProject: action.projects[0], loading: false}
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

  return state
}

export const RemailsContext = createContext<{ state: State, dispatch: ActionDispatch<[Action]> }>(
  {
    state: {
      organizations: [],
      projects: [],
      streams: [],
      loading: true
    },
    dispatch: () => {
      throw new Error('RemailsContext must be used within RemailsProvider');
    },
  });

export function useRemails() {
  return useContext(RemailsContext);
}

export function useLoadRemails() {
  const [state, dispatch] = useReducer(reducer, {
    organizations: [],
    projects: [],
    streams: [],
    loading: true
  });

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

  return {
    state,
    dispatch,
  };
}