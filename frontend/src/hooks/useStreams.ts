import { useRemails } from "./useRemails.ts";

export function useStreams() {
  const {
    state: { streams, routerState },
  } = useRemails();
  const currentStream = streams?.find((s) => s.id === routerState.params.stream_id) || null;

  return { streams, currentStream };
}
