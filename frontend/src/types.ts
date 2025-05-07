import {Route, RouteParams} from "./router.ts";

export type Role = 'super_admin' | { organization_admin: string };

export interface User {
  id: string
  roles: Role[];
  name: string;
  email: string;
  github_id: string | null;
  password_enabled: boolean;
}

export type WhoamiResponse = User | { error: string; }

export interface Message {
  id: string;
  from_email: string;
  created_at: string;
  recipients: string[];
  status: string;
}

export interface State {
  organizations: Organization[] | null;
  projects: Project[] | null;
  streams: Stream[] | null;
  messages: Message[] | null;
  domains: Domain[] | null;
  credentials: SmtpCredential[] | null;
  loading: boolean;

  // routing related state
  route: Route;
  fullPath: string;
  fullName: string;
  pathParams: RouteParams;
  queryParams: RouteParams;
}

export interface BreadcrumbItem {
  title: string;
  route: string;
  params?: RouteParams;
}

export type Action = {
  type: 'set_organizations';
  organizations: Organization[] | null;
} | {
  type: 'add_organization';
  organization: Organization;
} | {
  type: 'loading'
} | {
  type: 'set_projects';
  projects: Project[] | null;
} | {
  type: 'add_project';
  project: Project;
} | {
  type: 'remove_project';
  projectId: string;
} | {
  type: 'set_streams';
  streams: Stream[] | null;
} | {
  type: 'add_stream';
  stream: Stream;
} | {
  type: 'remove_stream';
  streamId: string;
} | {
  type: 'set_messages';
  messages: Message[] | null;
} | {
  type: 'set_domains';
  domains: Domain[] | null;
} | {
  type: 'add_domain';
  domain: Domain;
} | {
  type: 'remove_domain';
  domainId: string;
} | {
  type: 'set_credentials';
  credentials: SmtpCredential[] | null;
} | {
  type: 'add_credential';
  credential: SmtpCredential;
} | {
  type: 'remove_credential';
  credentialId: string;
} | {
  type: 'navigate';
  route: string;
  params?: RouteParams;
} | {
  type: 'set_route',
  route: Route;
  fullPath: string;
  fullName: string;
  pathParams: RouteParams;
  queryParams: RouteParams;
};

export interface Organization {
  id: string;
  name: string;
  created_at: string;
  updated_at: string;
}

export interface Project {
  id: string;
  name: string;
  created_at: string;
  updated_at: string;
}

export interface Stream {
  id: string;
  name: string;
  created_at: string;
  updated_at: string;
}

export interface Domain {
  id: string;
  parent_id: { organization: string } | { project: string }
  domain: string;
  dkim_key_type: 'rsa_sha265' | 'ed25519';
  dkim_public_key: string,
  created_at: string;
  updated_at: string;
}

export interface SmtpCredential {
  id: string;
  stream_id: string;
  description: string;
  username: string;
  created_at: string;
  updated_at: string;
}

export interface SmtpCredentialResponse extends SmtpCredential {
  cleartext_password: string;
}

export interface PasswordLoginRequest {
  email: string;
  password: string;
}

export interface SignUpRequest extends PasswordLoginRequest {
  name: string,
  terms: boolean,
}
