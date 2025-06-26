import { Action, State } from "./types";

const actionHandler: {
  [action in Action["type"]]: (state: State, action: Extract<Action, { type: action }>) => State;
} = {
  set_organizations: function (state, action) {
    return { ...state, organizations: action.organizations, loading: false };
  },
  add_organization: function (state, action) {
    return { ...state, organizations: [...(state.organizations || []), action.organization], loading: false };
  },
  loading: function (state) {
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
    return { ...state, routerState: action.routerState };
  },
  set_config: function (state, action) {
    return { ...state, config: action.config };
  },
};

// helper function to make TypeScript recognize the proper types
function getActionHandler<T extends Action["type"]>(
  action: Extract<Action, { type: T }>
): (state: State, action: Extract<Action, { type: T }>) => State {
  return actionHandler[action.type];
}

export function reducer(state: State, action: Action): State {
  console.log("action:", action);
  const handler = getActionHandler(action);
  return handler(state, action);
}
