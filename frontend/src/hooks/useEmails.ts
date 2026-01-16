import { useRemails } from "./useRemails.ts";
import { useEffect, useState } from "react";
import { useOrganizations } from "./useOrganizations.ts";
import { useProjects } from "./useProjects.ts";
import { Email, EmailMetadata } from "../types.ts";
import { RemailsError } from "../error/error.ts";
import { useSelector } from "./useSelector.ts";

export function useEmails() {
  const { currentOrganization } = useOrganizations();
  const { currentProject } = useProjects();
  const labels = useSelector((s) => s.labels || []);
  const [currentEmail, setCurrentEmail] = useState<Email | EmailMetadata | null>(null);
  const {
    state: { emails, routerState },
    dispatch,
  } = useRemails();

  useEffect(() => {
    if (routerState.params.email_id) {
      const partialEmail = emails?.find((m) => m.id === routerState.params.email_id) || null;
      setCurrentEmail(partialEmail);
      if (currentOrganization && currentProject) {
        fetch(
          `/api/organizations/${currentOrganization.id}/projects/${currentProject.id}/emails/${routerState.params.email_id}`
        )
          .then((res) => {
            if (res.ok) {
              return res.json();
            } else {
              const error = new RemailsError(
                `Could not load email with ID ${routerState.params.email_id} (${res.status} ${res.statusText})`,
                res.status
              );
              dispatch({ type: "set_error", error });
              throw error;
            }
          })
          .then((email) => {
            setCurrentEmail(email);
            dispatch({ type: "update_email", emailId: email.id, update: email });
          });
      }
    } else {
      setCurrentEmail(null);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentOrganization, currentProject, routerState.params.email_id]); // don't update on `emails`

  function updateEmail(email_id: string, update: Partial<Email>) {
    if (currentEmail?.id == email_id) {
      setCurrentEmail({ ...currentEmail, ...update });
    }

    dispatch({ type: "update_email", emailId: email_id, update: update });
  }

  return { emails, currentEmail, updateEmail, labels };
}
