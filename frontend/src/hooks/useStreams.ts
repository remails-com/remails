import {useRemails} from "./useRemails.ts";

export function useStreams() {
  const {state: {streams, params}} = useRemails()
  const currentStream = streams?.find((s) => s.id === params.stream_id) || null;

  return {streams, currentStream}
}
