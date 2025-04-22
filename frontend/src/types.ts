export type Role = 'super_admin' | { organization_admin: string };

export interface User {
  roles: Role[];
  name: string;
  email: string;
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
  organizations: Organization[];
  currentOrganization?: Organization;
  projects: Project[];
  streams: Stream[];
  loading: boolean;
}

export type Action = {
  type: 'set_organizations';
  organizations: Organization[];
} | {
  type: 'load_organizations'
} | {
  type: 'set_current_organization';
  organization: Organization;
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

export interface PasswordLoginRequest {
  email: string;
  password: string;
}

export interface SignUpRequest extends PasswordLoginRequest {
  name: string,
  terms: boolean,
}