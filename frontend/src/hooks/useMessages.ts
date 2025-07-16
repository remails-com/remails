import { useRemails } from "./useRemails.ts";
import { useEffect, useState } from "react";
import { useOrganizations } from "./useOrganizations.ts";
import { useProjects } from "./useProjects.ts";
import { useStreams } from "./useStreams.ts";
import { Message, MessageMetadata } from "../types.ts";

export function useMessages() {
  const { currentOrganization } = useOrganizations();
  const { currentProject } = useProjects();
  const { currentStream } = useStreams();
  const [currentMessage, setCurrentMessage] = useState<Message | MessageMetadata | null>(null);
  const {
    state: { messages, routerState },
  } = useRemails();

  useEffect(() => {
    if (routerState.params.message_id) {
      const incompleteMessage = messages?.find((m) => m.id === routerState.params.message_id) || null;
      setCurrentMessage(incompleteMessage);
      if (currentOrganization && currentProject && currentStream) {
        fetch(
          `/api/organizations/${currentOrganization.id}/projects/${currentProject.id}/streams/${currentStream.id}/messages/${routerState.params.message_id}`
        )
          .then((res) => res.json())
          .then(setCurrentMessage);
      }
    } else {
      setCurrentMessage(null);
    }
  }, [currentOrganization, currentProject, currentStream, routerState.params.message_id, messages]);

  function updateMessage(message_id: string, update: Partial<Message>) {
    if (currentMessage?.id == message_id) {
      console.log("Updating current message");
      setCurrentMessage({ ...currentMessage, ...update });
    } else {
      console.log("Updating other message");
    }
  }

  return { messages, currentMessage, updateMessage };
}
