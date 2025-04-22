import {ActionDispatch, createContext, useContext, useEffect, useReducer} from "react";
import {Action, State} from "../types.ts";

function reducer(state: State, action: Action): State {
  if (action.type === 'load_organizations') {
    return {...state, organizations: [], loading: true}
  }
  if (action.type === 'set_organizations') {
    return {...state, organizations: action.organizations, currentOrganization: action.organizations[0], loading: false}
  }
  if (action.type === 'set_current_organization') {
    return {...state, currentOrganization: action.organization}
  }

  return state
}

export const RemailsContext = createContext<{ state: State, dispatch: ActionDispatch<[Action]>}> (
  {
    state: {
      organizations: [],
      projects: [],
      streams: [],
      loading: true
    },
    dispatch: undefined,
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