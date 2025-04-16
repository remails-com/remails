import { useEffect, useState } from "react";
import { Organization } from "../types";

export function useOrganizations() {
  const [organizations, setOrganizations] = useState<Organization[]>([]);
  const [loading, setLoading] = useState(true);
  const [currentOrganization, setCurrentOrganization] = useState<Organization | null>(null);

  useEffect(() => {
    setLoading(true);
    fetch("/api/organizations")
      .then((res) => res.json())
      .then((data) => {
        if (Array.isArray(data)) {
          setOrganizations(data);
          // TODO store this somehow, e.g., as cookie or in local storage
          setCurrentOrganization(data[0]);
        }
        setLoading(false);
      });
  }, []);

  return { organizations, loading, currentOrganization, setCurrentOrganization }
}