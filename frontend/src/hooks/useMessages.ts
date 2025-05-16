import {useRemails} from "./useRemails.ts";
import {useEffect, useState} from "react";
import {useOrganizations} from "./useOrganizations.ts";
import {useProjects} from "./useProjects.ts";
import {useStreams} from "./useStreams.ts";
import {Message} from "postcss";
import {MessageMetadata} from "../types.ts";

export function useMessages() {
  const {currentOrganization} = useOrganizations();
  const {currentProject} = useProjects();
  const {currentStream} = useStreams();
  const [currentMessage, setCurrentMessage] = useState<Message | MessageMetadata | null>(null);
  const {state: {messages, pathParams}} = useRemails();

  useEffect(() => {
    if (pathParams.message_id) {
      const incompleteMessage = messages?.find((m) => m.id === pathParams.message_id) || null
      setCurrentMessage(incompleteMessage);
      if (currentOrganization && currentProject && currentStream) {
        fetch(`/api/organizations/${currentOrganization.id}/projects/${currentProject.id}/streams/${currentStream.id}/messages/${pathParams.message_id}`)
          .then(res => res.json())
          .then(setCurrentMessage)

      }
    } else {
      setCurrentMessage(null)
    }
  }, [currentOrganization, currentProject, currentStream, pathParams.message_id, messages]);

  return {messages, currentMessage}
}
