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
  loading: function (state, action) {
    return { ...state, loading: action.loading };
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
  set_domains: function (state, action) {
    return { ...state, domains: action.domains };
  },
  add_domain: function (state, action) {
    return { ...state, domains: [...(state.domains || []), action.domain] };
  },
  remove_domain: function (state, action) {
    return { ...state, domains: state.domains?.filter((d) => d.id !== action.domainId) || [] };
  },
  set_organisation_domains: function (state, action) {
    return { ...state, organisationDomains: action.organisationDomains };
  },
  add_organisation_domain: function (state, action) {
    return { ...state, organisationDomains: [...(state.organisationDomains || []), action.domain] };
  },
  remove_organisation_domain: function (state, action) {
    return { ...state, organisationDomains: state.organisationDomains?.filter((d) => d.id !== action.domainId) || [] };
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
  set_route: function (state, action) {
    return { ...state, routerState: action.routerState };
  },
  set_config: function (state, action) {
    return { ...state, config: action.config };
  },
  set_user: function (state, action) {
    return { ...state, user: action.user };
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
