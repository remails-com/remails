import { useOrganizations } from "../../hooks/useOrganizations.ts";
import { useProjects } from "../../hooks/useProjects.ts";
import { useRemails } from "../../hooks/useRemails.ts";
import { useDomains } from "../../hooks/useDomains.ts";
import { Domain, VerifyResult } from "../../types.ts";
import { modals } from "@mantine/modals";
import { notifications } from "@mantine/notifications";
import { IconInfoCircle, IconTrash, IconWorldWww } from "@tabler/icons-react";
import { Badge, Button, Code, Group, Loader, Paper, Popover, Table, Text, ThemeIcon, Tooltip } from "@mantine/core";
import { dkimRecord, dmarcValue, spfRecord } from "./DnsRecords.tsx";
import { useVerifyDomain } from "../../hooks/useVerifyDomain.ts";
import { formatDateTime } from "../../util.ts";
import { errorNotification } from "../../notify.tsx";
import Header from "../Header.tsx";
import { CopyableCode } from "../CopyableCode.tsx";
import React, { useState } from "react";
import { MaintainerButton } from "../RoleButtons.tsx";

const badgeColors: { [key in VerifyResult["status"]]: string } = {
  Success: "green",
  Info: "blue",
  Warning: "orange",
  Error: "remails-red",
};

function VerifyResultBadge({ verifyResult }: { verifyResult: VerifyResult | undefined }) {
  const [opened, setOpened] = useState(false);

  if (!verifyResult) {
    return <Loader color="gray" type="dots" size="sm" />;
  }

  if (verifyResult.status == "Success") {
    return <Badge color={badgeColors[verifyResult.status]}>OK</Badge>;
  }

  return (
    <Popover position="bottom" withArrow shadow="md" opened={opened} onChange={setOpened}>
      <Popover.Target>
        <Badge
          color={badgeColors[verifyResult.status]}
          component="button"
          style={{ cursor: "pointer" }}
          onClick={() => setOpened((o) => !o)}
          onMouseEnter={() => setOpened(true)}
          onMouseLeave={() => setOpened(false)}
        >
          {verifyResult.status}
        </Badge>
      </Popover.Target>
      <Popover.Dropdown maw="30em">
        <Text size="sm">
          <Text component="span" fw="bold">
            {verifyResult.status}:
          </Text>{" "}
          {verifyResult.reason}
        </Text>
        {verifyResult.value && (
          <>
            <Code block style={{ whiteSpace: "pre-wrap" }} my="2">
              {verifyResult?.value}
            </Code>
            <Text size="sm">Please verify this is configured as intended</Text>
          </>
        )}
      </Popover.Dropdown>
    </Popover>
  );
}

type DnsRow = {
  name: string;
  recordName: string;
  recordValue: React.ReactNode;
  recordType: string;
  verifyResult: VerifyResult | undefined;
};

function DnsTable({ rows }: { rows: DnsRow[] }) {
  return (
    <Table.ScrollContainer minWidth={640} mb="md">
      <Paper shadow="xs">
        <Table withTableBorder verticalSpacing="xs">
          <Table.Thead>
            <Table.Tr>
              <Table.Th style={{ width: "10%" }}></Table.Th>
              <Table.Th style={{ width: "25%" }}>Name</Table.Th>
              <Table.Th style={{ width: "10%" }}>Type</Table.Th>
              <Table.Th style={{ width: "45%" }}>Recommended value</Table.Th>
              <Table.Th style={{ width: "10%" }}>Status</Table.Th>
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>
            {rows.map(({ name, recordName, recordValue, recordType, verifyResult }) => (
              <Table.Tr key={name} bg={verifyResult?.status == "Error" ? "var(--mantine-color-remails-red-light)" : ""}>
                <Table.Th>{name}</Table.Th>
                <Table.Td>
                  <Code style={{ wordBreak: "break-word" }}>{recordName}</Code>
                </Table.Td>
                <Table.Td>{recordType}</Table.Td>
                <Table.Td>{recordValue}</Table.Td>
                <Table.Td>
                  <VerifyResultBadge verifyResult={verifyResult} />
                </Table.Td>
              </Table.Tr>
            ))}
          </Table.Tbody>
        </Table>
      </Paper>
    </Table.ScrollContainer>
  );
}

export default function DomainDetails() {
  const { currentOrganization } = useOrganizations();
  const { currentProject } = useProjects();
  const { currentDomain } = useDomains();
  const {
    dispatch,
    navigate,
    state: { config },
  } = useRemails();

  const domainsApi = currentProject
    ? `/api/organizations/${currentOrganization?.id}/projects/${currentProject.id}/domains`
    : `/api/organizations/${currentOrganization?.id}/domains`;
  const { reverifyDomain, domainVerified, verificationResult } = useVerifyDomain(domainsApi, currentDomain);

  if (!currentDomain || !currentOrganization || !config) {
    return null;
  }

  const confirmDeleteDomain = (domain: Domain) => {
    modals.openConfirmModal({
      title: "Please confirm your action",
      children: (
        <Text>
          Are you sure you want to delete the domain <strong>{domain.domain}</strong>? This action cannot be undone
        </Text>
      ),
      labels: { confirm: "Confirm", cancel: "Cancel" },
      onCancel: () => {},
      onConfirm: () => deleteDomain(domain),
    });
  };

  const domain_route = currentProject ? "projects.project.domains" : "domains";

  const deleteDomain = async (domain: Domain) => {
    const url = currentProject
      ? `/api/organizations/${currentOrganization.id}/projects/${currentProject.id}/domains/${domain.id}`
      : `/api/organizations/${currentOrganization.id}/domains/${domain.id}`;

    const res = await fetch(url, {
      method: "DELETE",
    });
    if (res.status === 200) {
      notifications.show({
        title: "Domain deleted",
        message: `Domain ${domain.domain} deleted`,
        color: "green",
      });
      navigate(domain_route);
      dispatch({ type: "remove_domain", domainId: domain.id, from_organization: !currentProject });
    } else {
      errorNotification(`Domain ${domain.domain} could not be deleted`);
      console.error(res);
    }
  };

  return (
    <>
      <Header
        name={currentDomain.domain}
        entityType={currentProject ? "Project Domain" : "Organization Domain"}
        Icon={IconWorldWww}
        divider
      />
      <h3>Required DNS records</h3>
      <DnsTable
        rows={[
          {
            name: "DKIM",
            recordName: `${config.dkim_selector}._domainkey.${currentDomain.domain}`,
            recordType: "TXT",
            recordValue: <CopyableCode>{dkimRecord(currentDomain)}</CopyableCode>,
            verifyResult: verificationResult?.dkim,
          },
          {
            name: "SPF",
            recordName: currentDomain.domain,
            recordType: "TXT",
            recordValue: <CopyableCode>{spfRecord(config)}</CopyableCode>,
            verifyResult: verificationResult?.spf,
          },
        ]}
      />

      <h3>Recommended DNS records</h3>
      <DnsTable
        rows={[
          {
            name: "DMARC",
            recordName: `_dmarc.${currentDomain.domain}`,
            recordType: "TXT",
            recordValue: <CopyableCode>{dmarcValue}</CopyableCode>,
            verifyResult: verificationResult?.dmarc,
          },
          {
            name: "A",
            recordName: currentDomain.domain,
            recordType: "TXT",
            recordValue: (
              <Group gap="xs">
                any
                <Tooltip label="Some mail services may require an A record to be set for the sender domain">
                  <ThemeIcon variant="transparent" c="dimmed" size="sm">
                    <IconInfoCircle />
                  </ThemeIcon>
                </Tooltip>
              </Group>
            ),
            verifyResult: verificationResult?.a,
          },
        ]}
      />

      <Text mt="md">
        Note that changes to DNS records may take some time to propagate. If verification fails, try again in a few
        minutes.
      </Text>
      <Text c="dimmed" size="sm">
        Last verified: {verificationResult ? formatDateTime(verificationResult?.timestamp) : "loading..."}
      </Text>
      <Group mt="xl" justify="space-between">
        <Button disabled={domainVerified === "loading"} onClick={() => reverifyDomain(currentDomain)}>
          Retry DNS verification
        </Button>
        <MaintainerButton
          leftSection={<IconTrash />}
          variant="outline"
          onClick={() => confirmDeleteDomain(currentDomain)}
        >
          Delete domain
        </MaintainerButton>
      </Group>
    </>
  );
}
