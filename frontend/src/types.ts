
export type User = {
  roles: any[];
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

export interface Organization {
  id: string;
  name: string;
  created_at: string;
  updated_at: string;
}