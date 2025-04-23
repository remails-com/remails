import {useEffect} from "react";
import {useRemails} from "./useRemails.ts";
import {useProjects} from "./useProjects.ts";
import {useRouter} from "./useRouter.ts";


export function useStreams() {
  const {currentProject} = useProjects();
  const {params} = useRouter();
  const {state: {currentOrganization, currentStream, streams}, dispatch} = useRemails()

  useEffect(() => {
    dispatch({type: 'load_streams'});

    if (!currentOrganization || !currentProject) {
      console.log("organization or project missing", currentOrganization, currentProject);
      return
    }

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
      const nextCurrentStream = streams.find((s) => s.id === params.stream_id);
      if (nextCurrentStream) {
        dispatch({type: "set_current_stream", stream: nextCurrentStream})
      }
    }
  }, [params.stream_id, streams]);

  return {streams, currentStream}
}