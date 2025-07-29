import { Box, Code, Flex, List, LoadingOverlay, Text, ThemeIcon } from "@mantine/core";
import { IconCheck, IconExclamationMark, IconInfoSmall, IconX } from "@tabler/icons-react";
import { DomainVerificationResult, DomainVerificationStatus, VerifyResult } from "../../types";
import { ReactElement } from "react";

export interface DnsVerificationProps {
  domain: string | undefined;
  domainVerified: DomainVerificationStatus;
  verificationResult: DomainVerificationResult | null;
}

const icons: { [key in VerifyResult["status"]]: ReactElement } = {
  Success: (
    <ThemeIcon color="teal" size={24} radius="xl">
      <IconCheck size={16} />
    </ThemeIcon>
  ),
  Info: (
    <ThemeIcon color="blue" size={24} radius="xl">
      <IconInfoSmall size={24} />
    </ThemeIcon>
  ),
  Warning: (
    <ThemeIcon color="orange" size={24} radius="xl" style={{ verticalAlign: "top" }}>
      <IconExclamationMark size={16} />
    </ThemeIcon>
  ),
  Error: (
    <ThemeIcon color="red" size={24} radius="xl">
      <IconX size={16} />
    </ThemeIcon>
  ),
};

function errWarn(result: VerifyResult | undefined) {
  if (!result) return "";

  if (result.status == "Error") {
    return " error";
  }

  if (result.status == "Warning") {
    return " warning";
  }

  return "";
}

export function DnsVerificationResult({ domainVerified, verificationResult }: DnsVerificationProps) {
  return (
    <Box pos="relative">
      <LoadingOverlay visible={domainVerified === "loading"} overlayProps={{ backgroundOpacity: 1 }} />
      <LoadingOverlay
        visible={domainVerified === "failed"}
        overlayProps={{ backgroundOpacity: 1 }}
        loaderProps={{
          children: (
            <Flex gap="md" justify="center" align="center">
              {icons["Error"]}
              <Text>DNS verification failed</Text>
            </Flex>
          ),
        }}
      />
      <List m="sm" spacing="md">
        <List.Item icon={icons[verificationResult?.dkim.status ?? "Warning"]}>
          DKIM{errWarn(verificationResult?.dkim)}: {verificationResult?.dkim.reason}
        </List.Item>
        <List.Item icon={icons[verificationResult?.spf.status ?? "Warning"]}>
          SPF{errWarn(verificationResult?.spf)}: {verificationResult?.spf.reason}
          {verificationResult?.spf?.value && (
            <>
              <Code block style={{ whiteSpace: "pre-wrap" }} my="2">
                {verificationResult?.spf?.value}
              </Code>
              <Text span fs="italic" c="dimmed">
                Please verify this is configured as intended
              </Text>
            </>
          )}
        </List.Item>
        <List.Item icon={icons[verificationResult?.dmarc.status ?? "Warning"]}>
          DMARC{errWarn(verificationResult?.dmarc)}: {verificationResult?.dmarc.reason}
          {verificationResult?.dmarc?.value && (
            <>
              <Code block style={{ whiteSpace: "pre-wrap" }} my="2">
                {verificationResult?.dmarc?.value}
              </Code>
              <Text span fs="italic" c="dimmed">
                Please verify this is configured as intended
              </Text>
            </>
          )}
        </List.Item>
        {verificationResult?.a.status != "Success" && (
          <List.Item icon={icons[verificationResult?.a.status ?? "Warning"]}>
            A record{errWarn(verificationResult?.a)}: {verificationResult?.a.reason}
            <Text fs="italic" c="dimmed">
              Some mail services may require an A record to be set for the sender domain
            </Text>
          </List.Item>
        )}
      </List>
    </Box>
  );
}
