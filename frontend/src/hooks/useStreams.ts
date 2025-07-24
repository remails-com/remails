import { RemailsError } from "../error/error.ts";
import { useSelector } from "./useSelector.ts";

export function useStreams() {
  const streams = useSelector((state) => state.streams || []);
  const routerState = useSelector((state) => state.routerState);
  const currentStream = streams?.find((s) => s.id === routerState.params.stream_id) || null;

  if (!currentStream && routerState.params.stream_id) {
    throw new RemailsError(`Could not find stream with ID ${routerState.params.stream_id}`, 404);
  }

  return { streams, currentStream };
}
