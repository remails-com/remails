import {useRemails} from "./useRemails.ts";

export function useStreams() {
  const {state: {streams, pathParams}} = useRemails()
  const currentStream = streams?.find((s) => s.id === pathParams.stream_id) || null;

  return {streams, currentStream}
}
