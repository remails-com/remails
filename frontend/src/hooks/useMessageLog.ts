import {useEffect, useState} from "react";
import {Message} from "../types";
import {useRouter} from "./useRouter.ts";
import {useOrganization} from "./useOrganizations.ts";

export function useMessageLog() {
  const [messages, setMessages] = useState<Message[]>([]);
  const [loading, setLoading] = useState(true);
  const {params, navigate} = useRouter();
  const {currentOrganization} = useOrganization();

  const org_id = currentOrganization?.id;
  const proj_id = params.proj_id;
  const stream_id = params.stream_id;

  useEffect(() => {
    setLoading(true);
    if (!(org_id && proj_id && stream_id)) {
      console.log(org_id, proj_id, stream_id);
      console.error("Missing org_id, proj_id, or stream_id");
      navigate('projects');
    }

    fetch(`/api/organizations/${org_id}/projects/${proj_id}/streams/${stream_id}/messages`)
      .then((res) => res.json())
      .then((data) => {
        if (Array.isArray(data)) {
          setMessages(data);
        }
        setLoading(false);
      });
  }, [org_id, proj_id, stream_id]);

  return {messages, loading}
}