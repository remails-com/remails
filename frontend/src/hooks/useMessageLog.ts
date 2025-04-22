import { useEffect, useState } from "react";
import { Message } from "../types";

export function useMessageLog() {
  const [messages, setMessages] = useState<Message[]>([]);
  const [loading, setLoading] = useState(true);
  
  useEffect(() => {
    setLoading(true);
    // FIXME: The URL changed as part of #88
    fetch("/api/messages")
      .then((res) => res.json())
      .then((data) => {
        if (Array.isArray(data)) {
          setMessages(data);
        }
        setLoading(false);
      });
  }, []);

  return { messages, loading }
}