import {useEffect} from "react";
import {useRemails} from "./useRemails.ts";
import {useProjects} from "./useProjects.ts";


export function useStreams() {
  const {currentProject} = useProjects();
  const {state: {currentOrganization, currentStream, streams, params}, dispatch} = useRemails()

  useEffect(() => {
    if (!currentOrganization || !currentProject) {
      console.error("organization or project missing", currentOrganization, currentProject);
      return
    }

    dispatch({type: 'load_streams'});

    fetch(`/api/organizations/${currentOrganization.id}/projects/${currentProject?.id}/streams`)
      .then((res) => res.json())
      .then((data) => {
        if (Array.isArray(data)) {
          dispatch({type: 'set_streams', streams: data});
        }
      });
  }, [currentOrganization, currentProject]);

  useEffect(() => {
    if (params.stream_id && streams) {
      if (params.stream_id === currentStream?.id) {
        return
      }

      const nextCurrentStream = streams.find((s) => s.id === params.stream_id);
      if (nextCurrentStream) {
        dispatch({type: "set_current_stream", stream: nextCurrentStream})
      }
    }
  }, [params.stream_id, streams]);

  return {streams, currentStream}
}