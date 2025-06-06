import { Code, Title, TitleOrder } from "@mantine/core";
import { Domain } from "../../types";
import { DnsRecord } from "./DnsRecord";

export function DnsRecords({
  domain,
  title_order = 3,
}: {
  domain: Domain | null;
  title_order: TitleOrder | undefined;
}) {
  const dkim_key_type = domain?.dkim_key_type == "ed25519" ? "ed25519" : "rsa";
  const dkim_entry = `v=DKIM1; k=${dkim_key_type}; p=${domain?.dkim_public_key}`;

  return (
    <>
      Please make sure to configure the following DNS records for the <Code>{domain?.domain}</Code> domain.
      <Title order={title_order} mt="md">
        1. DKIM Public Key
      </Title>
      Set a TXT record for <Code>remails._domainkey.{domain?.domain}</Code> to:
      <DnsRecord>{dkim_entry}</DnsRecord>
      <Title order={title_order} mt="md">
        2. Remails SPF
      </Title>
      Set a TXT record for <Code>{domain?.domain}</Code> to:
      <DnsRecord>v=spf1 include:remails.net -all</DnsRecord>
      <Title order={title_order} mt="md">
        3. DMARC Configuration
      </Title>
      Set a TXT record for <Code>_dmarc.{domain?.domain}</Code> to:
      <DnsRecord>v=DMARC1; p=reject</DnsRecord>
    </>
  );
}
