import { useRemails } from "./useRemails.ts";
import { useEffect, useState } from "react";
import { useOrganizations } from "./useOrganizations.ts";
import { Email, Suppressed, isFullEmail } from "../types.ts";
import { RemailsError } from "../error/error.ts";
import { useSelector } from "./useSelector.ts";
import { errorNotification } from "../notify.tsx";

export function useEmails() {
  const { currentOrganization } = useOrganizations();
  const labels = useSelector((s) => s.labels || []);
  const {
    state: { emails, routerState },
    dispatch,
  } = useRemails();
  const currentEmailId = routerState.params.email_id;
  const currentEmail = currentEmailId ? emails?.find((m) => m.id === currentEmailId) || null : null;

  useEffect(() => {
    if (!currentEmailId || !currentOrganization) {
      return;
    }

    if (isFullEmail(currentEmail)) {
      return;
    }

    fetch(`/api/organizations/${currentOrganization.id}/emails/${currentEmailId}`)
      .then((res) => {
        if (res.ok) {
          return res.json();
        } else {
          const error = new RemailsError(
            `Could not load email with ID ${currentEmailId} (${res.status} ${res.statusText})`,
            res.status
          );
          dispatch({ type: "set_error", error });
          throw error;
        }
      })
      .then((email) => {
        dispatch({ type: "update_email", emailId: email.id, update: email });
      });
  }, [currentEmail, currentEmailId, currentOrganization, dispatch]);

  function updateEmail(email_id: string, update: Partial<Email>) {
    dispatch({ type: "update_email", emailId: email_id, update: update });
  }

  return { emails, currentEmail, updateEmail, labels };
}

export function useSuppressed() {
  const { currentOrganization } = useOrganizations();
  const [suppressed, setSuppressed] = useState<Suppressed[] | null>(null);

  useEffect(() => {
    if (currentOrganization) {
      fetch(`/api/organizations/${currentOrganization.id}/emails/suppressed`)
        .then((res) => {
          if (res.status === 200) {
            return res.json();
          } else {
            errorNotification("Failed to load the suppressed email addresses");
            console.error(res);
            return null;
          }
        })
        .then(setSuppressed);
    }
  }, [currentOrganization]);

  return { suppressed, setSuppressed };
}
