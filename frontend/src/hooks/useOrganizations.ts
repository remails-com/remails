import { useEffect, useState } from "react";
import { Organization } from "../types";

export function useOrganizations() {
  const [organizations, setOrganizations] = useState<Organization[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    setLoading(true);
    fetch("/api/organizations")
      .then((res) => res.json())
      .then((data) => {
        if (Array.isArray(data)) {
          setOrganizations(data);
        }
        setLoading(false);
      });
  }, []);

  return { organizations, loading }
}