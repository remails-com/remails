import { useOrganizations } from "../../hooks/useOrganizations.ts";
import { useProjects } from "../../hooks/useProjects.ts";
import { useRemails } from "../../hooks/useRemails.ts";
import { useDomains } from "../../hooks/useDomains.ts";
import { Loader } from "../../Loader.tsx";
import { Domain } from "../../types.ts";
import { modals } from "@mantine/modals";
import { notifications } from "@mantine/notifications";
import { IconTrash, IconX } from "@tabler/icons-react";
import { Button, Container, Group, Modal, Stack, Text, TextInput, Title, Tooltip } from "@mantine/core";
import { DnsRecords } from "./DnsRecords.tsx";
import { DnsVerificationResult } from "./DnsVerificationResult.tsx";
import { useVerifyDomain } from "../../hooks/useVerifyDomain.tsx";
import { useDisclosure } from "@mantine/hooks";

export function DomainDetails() {
  const { currentOrganization } = useOrganizations();
  const { currentProject } = useProjects();
  const { currentDomain } = useDomains();
  const { dispatch, navigate } = useRemails();
  const [opened, { open, close }] = useDisclosure(false);

  const domainsApi = currentProject
    ? `/api/organizations/${currentOrganization?.id}/projects/${currentProject.id}/domains`
    : `/api/organizations/${currentOrganization?.id}/domains`;
  const { verifyDomain, domainVerified, verificationResult } = useVerifyDomain(domainsApi);

  if (!currentDomain || !currentOrganization) {
    return <Loader />;
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
    let url = `/api/organizations/${currentOrganization.id}/domains/${domain.id}`;
    if (currentProject) {
      url = `/api/organizations/${currentOrganization.id}/projects/${currentProject.id}/domains/${domain.id}`;
    }

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
      if (currentProject) {
        navigate("projects.project");
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
    <Container size="sm">
      <h2>Domain Details</h2>
      <Stack>
        <TextInput label="Domain" value={currentDomain.domain} readOnly={true} variant="filled" />
      </Stack>

      <Group mt="xl">
        <Tooltip label="Delete Domain">
          <Button leftSection={<IconTrash />} color="red" onClick={() => confirmDeleteDomain(currentDomain)}>
            Delete domain
          </Button>
        </Tooltip>
      </Group>

      <Title order={3} mt="xl">
        DNS configuration
      </Title>
      <DnsRecords domain={currentDomain} title_order={4}></DnsRecords>

      <Group mt="xl">
        <Tooltip label="Verify DNS records">
          <Button
            onClick={() => {
              verifyDomain(currentDomain);
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
        title={<Title order={2}>Verify DNS records</Title>}
        size="lg"
        padding="xl"
        centered
      >
        <DnsVerificationResult
          domain={currentDomain.domain}
          domainVerified={domainVerified}
          verificationResult={verificationResult}
        />
        <Group mt="md" justify="space-between">
          <Button
            disabled={domainVerified === "loading"}
            variant="outline"
            onClick={() => {
              verifyDomain(currentDomain);
            }}
          >
            Retry verification
          </Button>
          <Button onClick={close}>Done</Button>
        </Group>
      </Modal>
    </Container>
  );
}
