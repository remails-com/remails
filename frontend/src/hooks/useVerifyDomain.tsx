import { useCallback, useState } from "react";
import { Domain, DomainVerificationResult, DomainVerificationStatus } from "../types";
import { notifications } from "@mantine/notifications";
import { IconX } from "@tabler/icons-react";

export function useVerifyDomain(domainsApi: string) {
  const [domainVerified, setDomainVerified] = useState<DomainVerificationStatus>("loading");
  const [verificationResult, setVerificationResult] = useState<DomainVerificationResult | null>(null);

  const verifyDomain = useCallback(
    (newDomain: Domain | null) => {
      setVerificationResult(null);
      if (!newDomain) {
        setDomainVerified("failed");
        return;
      }
      setDomainVerified("loading");
      setTimeout(
        () =>
          fetch(`${domainsApi}/${newDomain.id}/verify`, {
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

  return { verifyDomain, domainVerified, verificationResult };
}
