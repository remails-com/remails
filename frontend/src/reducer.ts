import { Action, State } from "./types";

const actionHandler: {
  [action in Action["type"]]: (state: State, action: Extract<Action, { type: action }>) => State;
} = {
  set_organizations: function (state, action) {
    return { ...state, organizations: action.organizations };
  },
  add_organization: function (state, action) {
    return { ...state, organizations: [...(state.organizations || []), action.organization] };
  },
  set_projects: function (state, action) {
    return { ...state, projects: action.projects };
  },
  add_project: function (state, action) {
    return { ...state, projects: [...(state.projects || []), action.project] };
  },
  remove_project: function (state, action) {
    return { ...state, projects: state.projects?.filter((p) => p.id !== action.projectId) || [] };
  },
  set_streams: function (state, action) {
    return { ...state, streams: action.streams };
  },
  add_stream: function (state, action) {
    return { ...state, streams: [...(state.streams || []), action.stream] };
  },
  remove_stream: function (state, action) {
    return { ...state, streams: state.streams?.filter((p) => p.id !== action.streamId) || [] };
  },
  set_messages: function (state, action) {
    return { ...state, messages: action.messages };
  },
  update_message: function (state, action) {
    return {
      ...state,
      messages: state.messages?.map((m) => (m.id == action.messageId ? { ...m, ...action.update } : m)) ?? null,
    };
  },
  remove_message: function (state, action) {
    return { ...state, messages: state.messages?.filter((m) => m.id !== action.messageId) || [] };
  },
  set_domains: function (state, action) {
    return { ...state, domains: action.domains };
  },
  add_domain: function (state, action) {
    return { ...state, domains: [...(state.domains || []), action.domain] };
  },
  remove_domain: function (state, action) {
    return { ...state, domains: state.domains?.filter((d) => d.id !== action.domainId) || [] };
  },
  set_organization_domains: function (state, action) {
    return { ...state, organizationDomains: action.organizationDomains };
  },
  add_organization_domain: function (state, action) {
    return { ...state, organizationDomains: [...(state.organizationDomains || []), action.organizationDomain] };
  },
  remove_organization_domain: function (state, action) {
    return { ...state, organizationDomains: state.organizationDomains?.filter((d) => d.id !== action.domainId) || [] };
  },
  set_credentials: function (state, action) {
    return { ...state, credentials: action.credentials };
  },
  add_credential: function (state, action) {
    return { ...state, credentials: [...(state.credentials || []), action.credential] };
  },
  remove_credential: function (state, action) {
    return { ...state, credentials: state.credentials?.filter((d) => d.id !== action.credentialId) || [] };
  },
  set_next_router_state: function (state, action) {
    return { ...state, nextRouterState: action.nextRouterState };
  },
  set_route: function (state, action) {
    return { ...state, routerState: action.routerState, nextRouterState: null };
  },
  set_config: function (state, action) {
    return { ...state, config: action.config };
  },
  set_user: function (state, action) {
    return { ...state, user: action.user, userFetched: true };
  },
};

// helper function to make TypeScript recognize the proper types
function getActionHandler<T extends Action["type"]>(
  action: Extract<Action, { type: T }>
): (state: State, action: Extract<Action, { type: T }>) => State {
  return actionHandler[action.type];
}

export function reducer(state: State, action: Action): State {
  const handler = getActionHandler(action);
  const newState = handler(state, action);
  // console.log("action:", action, newState);
  return newState;
}
