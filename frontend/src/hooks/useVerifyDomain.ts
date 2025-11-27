import { useCallback, useState } from "react";
import { Domain, DomainVerificationResult, DomainVerificationStatus } from "../types";
import { errorNotification } from "../notify";
import { useOrganizations } from "./useOrganizations";

export function useVerifyDomain(domain: Domain | null) {
  const { currentOrganization } = useOrganizations();
  const [verificationResult, setVerificationResult] = useState<DomainVerificationResult | null>(
    domain?.verification_status ?? null
  );
  const [domainVerified, setDomainVerified] = useState<DomainVerificationStatus>(
    verificationResult ? "verified" : "loading"
  );

  const reverifyDomain = useCallback(
    (domain: Domain | null) => {
      setVerificationResult(null);
      if (!domain) {
        setDomainVerified("failed");
        return;
      }
      setDomainVerified("loading");
      setTimeout(
        () =>
          fetch(`/api/organizations/${currentOrganization?.id}/domains/${domain.id}/verify`).then((res) => {
            if (res.status !== 200) {
              errorNotification(`Domain ${domain.domain} could not be verified`);
              console.error(res);
              setDomainVerified("failed");
              return;
            }

            res.json().then((data) => {
              setDomainVerified("verified");
              setVerificationResult(data);
            });
          }),
        300
      );
    },
    [currentOrganization]
  );

  return { reverifyDomain, domainVerified, verificationResult };
}
