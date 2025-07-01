import { RouteParams, RouterState } from "./router";

export type Role = { type: "super_admin" } | { type: "organization_admin"; id: string };

export interface User {
  id: string;
  roles: Role[];
  name: string;
  email: string;
  github_id: string | null;
  password_enabled: boolean;
}

export type WhoamiResponse = User | { error: string };

export type DeliveryStatus = {
  type: "NotSent" | "Success" | "Reattempt" | "Failure";
  delivered?: string;
};

export interface MessageMetadata {
  id: string;
  from_email: string;
  created_at: string;
  recipients: string[];
  status: string;
  reason: string | undefined;
  raw_size: string;
  delivery_status: { [receiver: string]: DeliveryStatus };
  retry_after: string | undefined;
  attempts: number;
}

export interface Message extends MessageMetadata {
  message_data: {
    subject: string | null;
    date: string | null;
    text_body: string | null;
    attachments: {
      filename: string;
      mime: string;
      /** Human-readable size */
      size: string;
    }[];
  };
  truncated_raw_data: string;
  is_truncated: boolean;
}

export interface RemailsConfig {
  version: string;
  environment: string;
  smtp_domain_name: string;
  smtp_ports: number[];
  preferred_spf_record: string;
}

export interface State {
  user: User | null;
  userFetched: boolean;
  organizations: Organization[] | null;
  projects: Project[] | null;
  streams: Stream[] | null;
  messages: MessageMetadata[] | null;
  domains: Domain[] | null;
  organizationDomains: Domain[] | null;
  credentials: SmtpCredential[] | null;
  config: RemailsConfig | null;
  routerState: RouterState;
}

export interface BreadcrumbItem {
  title: string;
  route: string;
  params?: RouteParams;
}

export type Action =
  | {
      type: "set_user";
      user: User | null;
    }
  | {
      type: "set_organizations";
      organizations: Organization[] | null;
    }
  | {
      type: "add_organization";
      organization: Organization;
    }
  | {
      type: "set_projects";
      projects: Project[] | null;
    }
  | {
      type: "add_project";
      project: Project;
    }
  | {
      type: "remove_project";
      projectId: string;
    }
  | {
      type: "set_streams";
      streams: Stream[] | null;
    }
  | {
      type: "add_stream";
      stream: Stream;
    }
  | {
      type: "remove_stream";
      streamId: string;
    }
  | {
      type: "set_messages";
      messages: MessageMetadata[] | null;
    }
  | {
      type: "set_domains";
      domains: Domain[] | null;
    }
  | {
      type: "add_domain";
      domain: Domain;
    }
  | {
      type: "remove_domain";
      domainId: string;
    }
  | {
      type: "set_organization_domains";
      organizationDomains: Domain[] | null;
    }
  | {
      type: "add_organization_domain";
      organizationDomain: Domain;
    }
  | {
      type: "remove_organization_domain";
      domainId: string;
    }
  | {
      type: "set_credentials";
      credentials: SmtpCredential[] | null;
    }
  | {
      type: "add_credential";
      credential: SmtpCredential;
    }
  | {
      type: "remove_credential";
      credentialId: string;
    }
  | {
      type: "set_route";
      routerState: RouterState;
    }
  | {
      type: "set_config";
      config: RemailsConfig;
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
  parent_id: { organization: string } | { project: string };
  domain: string;
  dkim_key_type: "rsa_sha265" | "ed25519";
  dkim_public_key: string;
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
  name: string;
  terms: boolean;
}

export interface VerifyResult {
  status: "Success" | "Warning" | "Error";
  reason: string;
  value: string | null;
}

export interface DomainVerificationResult {
  dkim: VerifyResult;
  spf: VerifyResult;
  dmarc: VerifyResult;
  a: VerifyResult;
}

export type DomainVerificationStatus = "verified" | "failed" | "loading";
