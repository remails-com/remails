import {Route, RouteParams} from "./router.ts";

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
  organizations: Organization[] | null;
  projects: Project[] | null;
  streams: Stream[] | null;
  loading: boolean;

  // routing related state
  route: Route;
  fullPath: string;
  fullName: string;
  params: RouteParams;
  breadcrumbItems: BreadcrumbItem[];
}

export interface BreadcrumbItem {
  title: string;
  route: string;
}

export type Action = {
  type: 'set_organizations';
  organizations: Organization[];
} | {
  type: 'loading'
} | {
  type: 'set_projects';
  projects: Project[];
} | {
  type: 'set_streams';
  streams: Stream[];
} | {
  type: 'navigate';
  route: string;
  params?: RouteParams;
} | {
  type: 'set_route',
  route: Route;
  fullPath: string;
  fullName: string;
  params: RouteParams;
  breadcrumbItems: BreadcrumbItem[];
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
