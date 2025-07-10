import { useCallback, useState } from "react";
import { Domain, DomainVerificationResult, DomainVerificationStatus } from "../types";
import { notifications } from "@mantine/notifications";
import { IconX } from "@tabler/icons-react";

export function useVerifyDomain(domainsApi: string, domain: Domain | null) {
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
          fetch(`${domainsApi}/${domain.id}/verify`, {
            method: "POST",
          }).then((r) => {
            if (r.status !== 200) {
              notifications.show({
                title: "Error",
                message: `Something went wrong`,
                color: "red",
                autoClose: 20000,
                icon: <IconX size={20} />,
              });
              setDomainVerified("failed");
              return;
            }

            r.json().then((data) => {
              setDomainVerified("verified");
              setVerificationResult(data);
            });
          }),
        300
      );
    },
    [domainsApi]
  );

  return { reverifyDomain, domainVerified, verificationResult };
}
