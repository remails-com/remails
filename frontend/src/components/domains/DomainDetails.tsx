import { useOrganizations } from "../../hooks/useOrganizations.ts";
import { useProjects } from "../../hooks/useProjects.ts";
import { useRemails } from "../../hooks/useRemails.ts";
import { useDomains } from "../../hooks/useDomains.ts";
import { Domain } from "../../types.ts";
import { modals } from "@mantine/modals";
import { notifications } from "@mantine/notifications";
import { IconLabel, IconTrash, IconWorldSearch, IconX } from "@tabler/icons-react";
import { Button, Group, Modal, Stack, Text, TextInput, Title, Tooltip } from "@mantine/core";
import { DnsRecords } from "./DnsRecords.tsx";
import { DnsVerificationResult } from "./DnsVerificationResult.tsx";
import { useVerifyDomain } from "../../hooks/useVerifyDomain.tsx";
import { useDisclosure } from "@mantine/hooks";
import Tabs from "../../layout/Tabs.tsx";
import { DnsVerificationContent } from "./DnsVerificationContent.tsx";
import { formatDateTime } from "../../util.ts";

export function DomainDetails({ projectDomains = false }: { projectDomains?: boolean }) {
  const { currentOrganization } = useOrganizations();
  const { currentProject } = useProjects();
  const { currentDomain } = useDomains(projectDomains);
  const { dispatch, navigate } = useRemails();
  const [opened, { open, close }] = useDisclosure(false);

  const domainsApi =
    projectDomains && currentProject
      ? `/api/organizations/${currentOrganization?.id}/projects/${currentProject.id}/domains`
      : `/api/organizations/${currentOrganization?.id}/domains`;
  const { reverifyDomain, domainVerified, verificationResult } = useVerifyDomain(domainsApi, currentDomain);

  if (!currentDomain || !currentOrganization) {
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

  const deleteDomain = async (domain: Domain) => {
    const url =
      projectDomains && currentProject
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
      dispatch({ type: "remove_domain", domainId: domain.id });

      if (projectDomains && currentProject) {
        navigate("projects.project", { tab: "Domains" });
      } else {
        navigate("domains");
      }
    } else {
      notifications.show({
        title: "Error",
        message: `Domain ${domain.domain} could not be deleted`,
        color: "red",
        autoClose: 20000,
        icon: <IconX size={20} />,
      });
      console.error(res);
    }
  };

  return (
    <Tabs
      tabs={[
        {
          name: "Details",
          icon: <IconWorldSearch size={14} />,
          content: (
            <>
              <h2>Details</h2>
              <Stack>
                <TextInput label="Domain" value={currentDomain.domain} readOnly={true} variant="filled" />
              </Stack>
              <h3>DNS status</h3>
              <DnsVerificationResult
                domain={currentDomain.domain}
                domainVerified={domainVerified}
                verificationResult={verificationResult}
              />
              Last verified: {verificationResult ? formatDateTime(verificationResult?.timestamp) : "never"}
              <Group mt="xl">
                <Tooltip label="Delete Domain">
                  <Button
                    leftSection={<IconTrash />}
                    variant="outline"
                    onClick={() => confirmDeleteDomain(currentDomain)}
                  >
                    Delete domain
                  </Button>
                </Tooltip>
              </Group>
            </>
          ),
          notSoWide: true,
        },
        {
          name: "DNS records",
          icon: <IconLabel size={14} />,
          content: (
            <>
              <h2>DNS records</h2>
              <DnsRecords domain={currentDomain} title_order={4}></DnsRecords>

              <Group mt="xl">
                <Tooltip label="Verify DNS records">
                  <Button
                    onClick={() => {
                      reverifyDomain(currentDomain);
                      open();
                    }}
                  >
                    Verify DNS
                  </Button>
                </Tooltip>
              </Group>

              <Modal
                opened={opened}
                onClose={close}
                title={
                  <Title order={2} component="span">
                    Verify DNS records
                  </Title>
                }
                size="lg"
                padding="xl"
                centered
              >
                <DnsVerificationContent
                  domain={currentDomain.domain}
                  domainVerified={domainVerified}
                  verificationResult={verificationResult}
                />
                <Group mt="md" justify="space-between">
                  <Button
                    disabled={domainVerified === "loading"}
                    variant="outline"
                    onClick={() => reverifyDomain(currentDomain)}
                  >
                    Retry verification
                  </Button>
                  <Button onClick={close}>Done</Button>
                </Group>
              </Modal>
            </>
          ),
          notSoWide: true,
        },
      ]}
    />
  );
}
