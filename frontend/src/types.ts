import {Navigate, Route, RouteParams} from "./hooks/useRouter.ts";

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
  currentProject?: Project;
  streams: Stream[];
  currentStream?: Stream;
  loading: boolean;

  // routing related state
  route: Route;
  fullPath: string;
  fullName: string;
  params: RouteParams;
  breadcrumbItems: BreadcrumbItem[];
  navigate: Navigate;
}

export interface BreadcrumbItem  {
  title: string;
  route: string;
}

export type Action = {
  type: 'set_organizations';
  organizations: Organization[];
} | {
  type: 'load_organizations'
} | {
  type: 'set_current_organization';
  organization: Organization;
} | {
  type: 'load_projects'
} | {
  type: 'set_current_project';
  project: Project;
} | {
  type: 'set_projects';
  projects: Project[];
} | {
  type: 'set_streams';
  streams: Stream[];
} | {
  type: 'load_streams'
} | {
  type: 'set_current_stream';
  stream: Stream;
} | {
  type: 'navigate';
  route: string;
  params?: RouteParams;
}
  ;

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