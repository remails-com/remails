import { RemailsError } from "./error/error";
import { RouterState } from "./router";

export type Role = "admin" | "maintainer" | "read_only";
export type OrgRole = { role: Role; org_id: string };

export interface User {
  id: string;
  global_role: Role | null;
  org_roles: OrgRole[];
  name: string;
  email: string;
  github_id: string | null;
  password_enabled: boolean;
}

export interface TotpCode {
  id: string;
  description: string;
  last_used: string;
}

export type WhoamiResponse =
  | (User & { login_status: "logged_in" })
  | { login_status: "mfa_pending" }
  | { error: string };

export type DeliveryStatus =
  | {
      type: "Success";
      delivered: string;
    }
  | {
      type: "NotSent" | "Reattempt" | "Failed";
    };

export interface DeliveryDetails {
  status: DeliveryStatus;
  log: Log;
}

export interface Log {
  lines: Array<{
    time: string;
    level: string;
    msg: string;
  }>;
}

export interface MessageMetadata {
  id: string;
  from_email: string;
  created_at: string;
  recipients: string[];
  status: "Processing" | "Held" | "Accepted" | "Rejected" | "Delivered" | "Reattempt" | "Failed";
  reason: string | undefined;
  raw_size: string;
  message_id_header: string;
  delivery_details: { [receiver: string]: DeliveryDetails };
  retry_after: string | undefined;
  attempts: number;
  max_attempts: number;
  label: string | undefined;
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
  spf_include: string;
  dkim_selector: string;
  moneybird_administration_id: string;
}

export interface RuntimeConfig {
  system_email_project: string;
  system_email_project_name: string;
  system_email_address: string;
  system_email_organization: string;
}

export interface State {
  user: User | null;
  userFetched: boolean;
  totpCodes: TotpCode[] | null;
  organizations: Organization[] | null;
  projects: Project[] | null;
  labels: string[] | null;
  messages: MessageMetadata[] | null;
  domains: Domain[] | null;
  credentials: SmtpCredential[] | null;
  apiKeys: ApiKey[] | null;
  config: RemailsConfig | null;
  runtimeConfig: RuntimeConfig | null;
  routerState: RouterState;
  nextRouterState: RouterState | null;
  error: RemailsError | null;
}

export type Action =
  | {
      type: "set_user";
      user: User | null;
    }
  | {
      type: "set_totp_codes";
      totpCodes: TotpCode[] | null;
    }
  | {
      type: "remove_totp_code";
      totpCodeId: string;
    }
  | {
      type: "add_totp_code";
      totpCode: TotpCode;
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
      type: "remove_organization";
      organizationId: string;
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
      type: "set_labels";
      labels: string[] | null;
    }
  | {
      type: "set_messages";
      messages: MessageMetadata[] | null;
    }
  | {
      type: "update_message";
      messageId: string;
      update: Partial<Message>;
    }
  | {
      type: "remove_message";
      messageId: string;
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
      type: "set_api_keys";
      apiKeys: ApiKey[] | null;
    }
  | {
      type: "add_api_key";
      apiKey: ApiKey;
    }
  | {
      type: "remove_api_key";
      apiKeyId: string;
    }
  | {
      type: "set_next_router_state";
      nextRouterState: RouterState | null;
    }
  | {
      type: "set_route";
      routerState: RouterState;
    }
  | {
      type: "set_config";
      config: RemailsConfig;
    }
  | {
      type: "set_runtime_config";
      config: RuntimeConfig;
    }
  | {
      type: "set_subscription";
      status: SubscriptionStatus;
      organizationId: string;
    }
  | {
      type: "set_error";
      error: RemailsError;
    };

export type OrgBlockStatus = "not_blocked" | "no_sending" | "no_sending_or_receiving";

export type PasswordResetState = "NotActive" | "ActiveWithout2Fa" | "ActiveWith2Fa";

export interface Organization {
  id: string;
  name: string;
  total_message_quota: number;
  used_message_quota: number;
  current_subscription: SubscriptionStatus;
  quota_reset: string | null;
  moneybird_contact_id: string | null;
  created_at: string;
  updated_at: string;
  block_status: OrgBlockStatus;
}

export interface Project {
  id: string;
  name: string;
  created_at: string;
  updated_at: string;
}

export interface VerifyResult {
  status: "Success" | "Info" | "Warning" | "Error";
  reason: string;
  value: string | null;
}

export interface DomainVerificationResult {
  timestamp: string;
  dkim: VerifyResult;
  spf: VerifyResult;
  dmarc: VerifyResult;
  a: VerifyResult;
}

export type DomainVerificationStatus = "verified" | "failed" | "loading";

export interface Domain {
  id: string;
  organization_id: string;
  project_id: string | null;
  domain: string;
  dkim_key_type: "rsa_sha265" | "ed25519";
  dkim_public_key: string;
  verification_status: DomainVerificationResult | null;
  created_at: string;
  updated_at: string;
}

export interface SmtpCredential {
  id: string;
  project_id: string;
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
}

export type ProductIdentifier =
  | "RMLS-FREE"
  | "RMLS-TINY-MONTHLY"
  | "RMLS-SMALL-MONTHLY"
  | "RMLS-MEDIUM-MONTHLY"
  | "RMLS-LARGE-MONTHLY"
  | "RMLS-TINY-YEARLY"
  | "RMLS-SMALL-YEARLY"
  | "RMLS-MEDIUM-YEARLY"
  | "RMLS-LARGE-YEARLY";

export type SubscriptionStatus =
  | (Subscription & { status: "active" })
  | (Subscription & { status: "expired"; end_date: string })
  | { status: "none" };

export interface Subscription {
  subscription_id: string;
  product: ProductIdentifier;
  title: string;
  description: string;
  recurring_sales_invoice_id: string;
  start_date: string;
  end_date: string | null;
  sales_invoices_url: string;
}

export type Invite = {
  id: string;
  organization_id: string;
  organization_name: string;
  role: Role;
  created_by: string;
  created_by_name: string;
  created_at: string;
  expires_at: string;
};

export type CreatedInvite = {
  id: string;
  password: string;
  organization_id: string;
  created_by: string;
  created_at: string;
  expires_at: string;
};

export type OrganizationMember = {
  user_id: string;
  email: string;
  name: string;
  role: Role;
  added_at: string;
  updated_at: string;
};

export type KeyRole = Exclude<Role, "admin">;

export type ApiKey = {
  id: string;
  description: string;
  organization_id: string;
  role: KeyRole;
  created_at: string;
  updated_at: string;
};

export type CreatedApiKeyWithPassword = ApiKey & { password: string };
