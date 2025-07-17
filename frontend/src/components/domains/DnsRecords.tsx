import { Code, Title, TitleOrder } from "@mantine/core";
import { Domain } from "../../types";
import { CopyableCode } from "../CopyableCode";
import { useRemails } from "../../hooks/useRemails";

export function DnsRecords({
  domain,
  title_order = 3,
}: {
  domain: Domain | null;
  title_order: TitleOrder | undefined;
}) {
  const {
    state: { config },
  } = useRemails();

  if (!config || !domain) {
    return null;
  }

  const dkim_key_type = domain.dkim_key_type == "ed25519" ? "ed25519" : "rsa";
  const dkim_entry = `v=DKIM1; k=${dkim_key_type}; p=${domain.dkim_public_key}`;

  return (
    <>
      Please make sure to configure the following DNS records for the <Code>{domain.domain}</Code> domain.
      <Title order={title_order} mt="md">
        1. DKIM Public Key
      </Title>
      Set a TXT record for{" "}
      <Code>
        {config.dkim_selector}._domainkey.{domain.domain}
      </Code>{" "}
      to:
      <CopyableCode mt="xs">{dkim_entry}</CopyableCode>
      <Title order={title_order} mt="md">
        2. Remails SPF
      </Title>
      Set a TXT record for <Code>{domain.domain}</Code> to:
      <CopyableCode mt="xs">{config.preferred_spf_record ?? ""}</CopyableCode>
      <Title order={title_order} mt="md">
        3. DMARC Configuration
      </Title>
      Set a TXT record for <Code>_dmarc.{domain.domain}</Code> to:
      <CopyableCode mt="xs">v=DMARC1; p=reject</CopyableCode>
    </>
  );
}
