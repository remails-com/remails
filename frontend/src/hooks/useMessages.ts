import {useRemails} from "./useRemails.ts";

export function useMessages() {
  const {state: {messages, pathParams}} = useRemails();
  const currentMessage = messages?.find((m) => m.id === pathParams.message_id) || null;

  return {messages, currentMessage}
}
