import { Button, Group, Modal, Stack, Stepper, TextInput, Title } from "@mantine/core";
import { useForm } from "@mantine/form";
import { useOrganizations } from "../../hooks/useOrganizations.ts";
import { useRemails } from "../../hooks/useRemails.ts";
import { IconX } from "@tabler/icons-react";
import { notifications } from "@mantine/notifications";
import { useEffect, useState } from "react";
import { Domain } from "../../types.ts";
import { useProjects } from "../../hooks/useProjects.ts";
import { DnsRecords } from "./DnsRecords.tsx";
import { useVerifyDomain } from "../../hooks/useVerifyDomain.tsx";
import { DnsVerificationContent } from "./DnsVerificationContent.tsx";

interface FormValues {
  domain: string;
}

interface NewDomainProps {
  opened: boolean;
  close: () => void;
  projectId: string | null;
}

function validateDomain(domain: string) {
  if (domain.length < 3) {
    return "Domain must have at least 3 letters";
  }

  if (!domain.includes(".")) {
    return "Domain must include a top level domain (TLD)";
  }

  return null;
}

export function NewDomain({ opened, close, projectId }: NewDomainProps) {
  const [activeStep, setActiveStep] = useState(0);
  const { currentOrganization } = useOrganizations();
  const { currentProject } = useProjects();
  const [newDomain, setNewDomain] = useState<Domain | null>(null);
  const { navigate, dispatch } = useRemails();

  const domainsApi = projectId
    ? `/api/organizations/${currentOrganization?.id}/projects/${projectId}/domains`
    : `/api/organizations/${currentOrganization?.id}/domains`;

  const { reverifyDomain: verifyDomain, domainVerified, verificationResult } = useVerifyDomain(domainsApi, newDomain);

  const form = useForm<FormValues>({
    initialValues: {
      domain: "",
    },
    validate: {
      domain: validateDomain,
    },
  });

  useEffect(() => {
    if (activeStep === 2) {
      verifyDomain(newDomain);
    }
  }, [activeStep, newDomain, verifyDomain]);

  if (!currentOrganization) {
    console.error("Cannot create domain without a selected organization");
    return null;
  }

  const save = (values: FormValues) => {
    fetch(domainsApi, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ ...values, dkim_key_type: "rsa_sha256" }),
    }).then((res) => {
      if (res.status === 201) {
        res.json().then((newDomain) => {
          setNewDomain(newDomain);

          if (projectId) {
            dispatch({ type: "add_domain", domain: newDomain });
          } else {
            dispatch({ type: "add_organization_domain", organizationDomain: newDomain });
          }

          setActiveStep(1);
        });
      } else if (res.status === 409) {
        form.setFieldError("domain", "This domain is already configured");
      } else {
        notifications.show({
          title: "Error",
          message: "Something went wrong",
          color: "red",
          autoClose: 20000,
          icon: <IconX size={20} />,
        });
      }
    });
  };

  const deleteDomain = (domain: Domain | null) => {
    if (!domain) {
      return;
    }

    fetch(`${domainsApi}/${domain.id}`, {
      method: "DELETE",
    }).then((r) => {
      if (projectId) {
        dispatch({ type: "remove_domain", domainId: domain.id });
      } else {
        dispatch({ type: "remove_organization_domain", domainId: domain.id });
      }

      if (r.status !== 200) {
        notifications.show({
          title: "Error",
          message: `Something went wrong`,
          color: "red",
          autoClose: 20000,
          icon: <IconX size={20} />,
        });
      }
    });
  };

  return (
    <>
      <Modal
        opened={opened}
        onClose={() => {
          setActiveStep(0);
          form.reset();
          close();
        }}
        title={
          <Title order={2} component="span">
            Create New Domain
          </Title>
        }
        size="lg"
        padding="xl"
      >
        <Stepper active={activeStep} onStepClick={setActiveStep}>
          <Stepper.Step label="Create" allowStepSelect={false}>
            <form onSubmit={form.onSubmit(save)}>
              <Stack>
                <TextInput
                  label="Domain Name"
                  key={form.key("domain")}
                  value={form.values.domain}
                  placeholder="example.com"
                  error={form.errors.domain}
                  onChange={(event) => form.setFieldValue("domain", event.currentTarget.value)}
                />
              </Stack>

              <Group justify="space-between" mt="xl">
                <Button onClick={close} variant="outline">
                  Cancel
                </Button>
                <Button type="submit" loading={form.submitting}>
                  Next
                </Button>
              </Group>
            </form>
          </Stepper.Step>
          <Stepper.Step label="Configure DNS" allowStepSelect={activeStep >= 1}>
            <DnsRecords domain={newDomain} title_order={3}></DnsRecords>
            <Group justify="space-between" mt="md">
              <Button
                variant="outline"
                onClick={() => {
                  setActiveStep(0);
                  deleteDomain(newDomain);
                  close();
                }}
              >
                Cancel
              </Button>
              <Group>
                <Button
                  onClick={() => {
                    setActiveStep(0);
                    form.reset();
                    close();
                  }}
                >
                  Configure later
                </Button>
                <Button
                  onClick={() => {
                    setActiveStep(2);
                  }}
                >
                  Verify
                </Button>
              </Group>
            </Group>
          </Stepper.Step>
          <Stepper.Step label="Verify" allowStepSelect={activeStep >= 1}>
            <DnsVerificationContent
              domainVerified={domainVerified}
              verificationResult={verificationResult}
              domain={newDomain?.domain}
            />
            <Group justify="space-between" mt="md">
              <Button disabled={domainVerified === "loading"} variant="outline" onClick={() => verifyDomain(newDomain)}>
                Retry verification
              </Button>
              <Button
                disabled={domainVerified === "loading"}
                onClick={() => {
                  setActiveStep(0);
                  close();
                  const route = currentProject ? "projects.project.domains.domain" : "domains.domain";
                  navigate(route, { domain_id: newDomain?.id || "" });
                }}
              >
                Show {newDomain?.domain}
              </Button>
            </Group>
          </Stepper.Step>
        </Stepper>
      </Modal>
    </>
  );
}
