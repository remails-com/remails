import { useRemails } from "./useRemails.ts";
import { useEffect, useState } from "react";
import { useOrganizations } from "./useOrganizations.ts";
import { useProjects } from "./useProjects.ts";
import { Message, MessageMetadata } from "../types.ts";
import { RemailsError } from "../error/error.ts";
import { useSelector } from "./useSelector.ts";

export function useMessages() {
  const { currentOrganization } = useOrganizations();
  const { currentProject } = useProjects();
  const labels = useSelector((s) => s.labels || []);
  const [currentMessage, setCurrentMessage] = useState<Message | MessageMetadata | null>(null);
  const {
    state: { messages, routerState },
    dispatch,
  } = useRemails();

  useEffect(() => {
    if (routerState.params.message_id) {
      const incompleteMessage = messages?.find((m) => m.id === routerState.params.message_id) || null;
      setCurrentMessage(incompleteMessage);
      if (currentOrganization && currentProject) {
        fetch(
          `/api/organizations/${currentOrganization.id}/projects/${currentProject.id}/messages/${routerState.params.message_id}`
        )
          .then((res) => {
            if (res.ok) {
              return res.json();
            } else {
              const error = new RemailsError(
                `Could not load message with ID ${routerState.params.message_id} (${res.status} ${res.statusText})`,
                res.status
              );
              dispatch({ type: "set_error", error });
              throw error;
            }
          })
          .then((message) => {
            setCurrentMessage(message);
            dispatch({ type: "update_message", messageId: message.id, update: message });
          });
      }
    } else {
      setCurrentMessage(null);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentOrganization, currentProject, routerState.params.message_id]); // don't update on messages

  function updateMessage(message_id: string, update: Partial<Message>) {
    if (currentMessage?.id == message_id) {
      setCurrentMessage({ ...currentMessage, ...update });
    }

    dispatch({ type: "update_message", messageId: message_id, update: update });
  }

  return { messages, currentMessage, updateMessage, labels };
}
