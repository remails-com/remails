import {createContext, useContext, useEffect, useState} from "react";
import {Organization,} from "../types";

export interface OrganizationContextProps {
  currentOrganization?: Organization;
  setCurrentOrganization: (currentOrganization: Organization) => void;
  organizations: Organization[];
}

export const OrganizationContext = createContext<OrganizationContextProps | null>(null);

export function useOrganization(): OrganizationContextProps {
  const organization = useContext(OrganizationContext);

  if (!organization) {
    throw new Error("useOrganization must be used within a OrganizationProvider");
  }

  return organization;
}

export function useLoadOrganizations() {
  const [organizations, setOrganizations] = useState<Organization[]>([]);
  const [loading, setLoading] = useState(true);
  const [currentOrganization, setCurrentOrganization] = useState<Organization | undefined>(undefined);

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

  return {organizations, loading, currentOrganization, setCurrentOrganization}
}