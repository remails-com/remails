import { useRemails } from "./useRemails.ts";

export function useStreams() {
  const {
    state: { streams, routerState },
    navigate,
  } = useRemails();
  const currentStream = streams?.find((s) => s.id === routerState.params.stream_id) || null;

  if (!currentStream && routerState.params.stream_id) {
    navigate("not_found");
  }

  return { streams, currentStream };
}
