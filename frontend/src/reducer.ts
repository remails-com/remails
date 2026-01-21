import { Action, State } from "./types";

const actionHandler: {
  [action in Action["type"]]: (state: State, action: Extract<Action, { type: action }>) => State;
} = {
  set_organizations: function (state, action) {
    return { ...state, organizations: action.organizations };
  },
  set_api_users: function (state, action) {
    return { ...state, apiUsers: action.users };
  },
  set_api_user_role: function (state, action) {
    return {
      ...state,
      apiUsers:
        state.apiUsers?.map((u) => {
          if (u.id === action.user_id) {
            u.global_role = action.role;
          }
          return u;
        }) || [],
    };
  },
  add_organization: function (state, action) {
    return { ...state, organizations: [action.organization, ...(state.organizations || [])] };
  },
  remove_organization: function (state, action) {
    return { ...state, organizations: state.organizations?.filter((o) => o.id !== action.organizationId) || [] };
  },
  set_projects: function (state, action) {
    return { ...state, projects: action.projects };
  },
  add_project: function (state, action) {
    return { ...state, projects: [action.project, ...(state.projects || [])] };
  },
  remove_project: function (state, action) {
    return { ...state, projects: state.projects?.filter((p) => p.id !== action.projectId) || [] };
  },
  set_labels: function (state, action) {
    return { ...state, labels: action.labels };
  },
  set_emails: function (state, action) {
    return { ...state, emails: action.emailMetadata };
  },
  update_email: function (state, action) {
    return {
      ...state,
      emails: state.emails?.map((m) => (m.id == action.emailId ? { ...m, ...action.update } : m)) ?? null,
    };
  },
  remove_email: function (state, action) {
    return { ...state, emails: state.emails?.filter((m) => m.id !== action.emailId) || [] };
  },
  set_domains: function (state, action) {
    return { ...state, domains: action.domains };
  },
  add_domain: function (state, action) {
    return { ...state, domains: [action.domain, ...(state.domains || [])] };
  },
  remove_domain: function (state, action) {
    return { ...state, domains: state.domains?.filter((d) => d.id !== action.domainId) || [] };
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
  set_api_keys: function (state, action) {
    return { ...state, apiKeys: action.apiKeys };
  },
  add_api_key: function (state, action) {
    return { ...state, apiKeys: [...(state.apiKeys || []), action.apiKey] };
  },
  remove_api_key: function (state, action) {
    return { ...state, apiKeys: state.apiKeys?.filter((k) => k.id !== action.apiKeyId) || [] };
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
  set_runtime_config: function (state, action) {
    return { ...state, runtimeConfig: action.config };
  },
  set_user: function (state, action) {
    return { ...state, user: action.user, userFetched: true };
  },
  set_totp_codes: (state, action) => {
    return { ...state, totpCodes: action.totpCodes };
  },
  remove_totp_code: (state, action) => {
    return { ...state, totpCodes: state.totpCodes?.filter((c) => c.id !== action.totpCodeId) || null };
  },
  add_totp_code: (state, action) => {
    return { ...state, totpCodes: [action.totpCode, ...(state.totpCodes || [])] };
  },
  set_subscription: function (state, action) {
    const org = state.organizations?.find((o) => o.id === action.organizationId);
    if (!org) {
      console.error("Cannot find organization to update subscription");
      return state;
    }
    org.current_subscription = action.status;
    return {
      ...state,
      organizations: [...(state.organizations?.filter((o) => o.id !== action.organizationId) || []), org],
    };
  },
  set_statistics: function (state, action) {
    return { ...state, statistics: action.statistics };
  },
  set_error: function (state, action) {
    return { ...state, error: action.error };
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
