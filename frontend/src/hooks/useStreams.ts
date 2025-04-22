import {createContext, useContext, useEffect, useState} from "react";
import {Organization, Project, Stream,} from "../types";

export interface StreamContextProps {
  currentStream?: Stream;
  setCurrentStream: (currentStream: Stream) => void;
  streams: Stream[];
  loading: boolean;
}

export const StreamContext = createContext<StreamContextProps | null>(null);

export function useStreams(): StreamContextProps {
  const streams = useContext(StreamContext);

  if (!streams) {
    throw new Error("useStreams must be used within a StreamProvider");
  }

  return streams;
}

export function useLoadStreams(currentOrganization?: Organization, currentProject?: Project) {
  const [streams, setStreams] = useState<Stream[]>([]);
  const [loading, setLoading] = useState(true);
  const [currentStream, setCurrentStream] = useState<Stream | undefined>(undefined);

  useEffect(() => {
    setLoading(true);

    if (!currentOrganization || !currentProject) {
      return
    }

    fetch(`/api/organizations/${currentOrganization.id}/projects/${currentProject?.id}/streams`)
      .then((res) => res.json())
      .then((data) => {
        if (Array.isArray(data)) {
          setStreams(data);
          setCurrentStream(data[0]);
        }
        setLoading(false);
      });
  }, [currentOrganization, currentProject]);

  return {streams, loading, currentStream, setCurrentStream}
}