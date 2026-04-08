import { useEffect, useState } from "react";
import { useOrganizations } from "./useOrganizations";
import { AuditLogEntry } from "../types";
import { errorNotification } from "../notify";

export function useAuditLogEntries() {
  const { currentOrganization } = useOrganizations();
  const [auditLogEntries, setAuditLogEntries] = useState<AuditLogEntry[] | null>(null);

  useEffect(() => {
    if (currentOrganization) {
      fetch(`/api/organizations/${currentOrganization.id}/audit-log`)
        .then((res) => {
          if (res.status === 200) {
            return res.json();
          } else {
            errorNotification("Failed to load the audit log entries");
            console.error(res);
            return null;
          }
        })
        .then(setAuditLogEntries);
    }
  }, [currentOrganization]);

  return { auditLogEntries, setAuditLogEntries };
}
