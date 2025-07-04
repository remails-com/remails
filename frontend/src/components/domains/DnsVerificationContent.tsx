import { Code, Text, Title } from "@mantine/core";
import { DnsVerificationProps, DnsVerificationResult } from "./DnsVerificationResult";

export function DnsVerificationContent({ domain, domainVerified, verificationResult }: DnsVerificationProps) {
  return (
    <>
      We will verify for you if the DNS of <Code>{domain}</Code> has been configured correctly.
      <Title order={3} mt="md">
        DNS verification results
      </Title>
      <DnsVerificationResult domain={domain} domainVerified={domainVerified} verificationResult={verificationResult} />
      <Text>
        Note that changes to DNS records may take some time to propagate. If verification fails, try again in a few
        minutes.
      </Text>
    </>
  );
}
