import { Button, Group, Modal, MultiSelect, Stack, Stepper, TextInput, Title } from "@mantine/core";
import { useForm } from "@mantine/form";
import { useOrganizations } from "../../hooks/useOrganizations.ts";
import { useRemails } from "../../hooks/useRemails.ts";
import { useEffect, useState } from "react";
import { Domain } from "../../types.ts";
import { DnsRecords } from "./DnsRecords.tsx";
import { useVerifyDomain } from "../../hooks/useVerifyDomain.ts";
import { DnsVerificationContent } from "./DnsVerificationContent.tsx";
import { errorNotification } from "../../notify.tsx";
import { useProjects } from "../../hooks/useProjects.ts";

interface FormValues {
  domain: string;
  project_ids: string[];
}

interface NewDomainProps {
  opened: boolean;
  close: () => void;
}

function validateDomain(domain: string) {
  if (domain.length < 3) {
    return "Domain must have at least 3 characters";
  }

  if (!domain.includes(".")) {
    return "Domain must include a top level domain (TLD)";
  }

  if (domain.startsWith(".") || domain.endsWith(".")) {
    return "Domain must not start or end with a dot";
  }

  if (domain.includes("..")) {
    return "Domain must not contain consecutive dots";
  }

  if (domain.includes("://")) {
    return 'Domain must not include a URL protocol (use "example.com" instead of "https://example.com")';
  }

  if (domain.includes("/")) {
    return "Domain must not include a URL path";
  }

  if (domain.includes("#")) {
    return "Domain must not include a URL hash";
  }

  if (domain.includes("?")) {
    return "Domain must not include any URL search parameters";
  }

  return null;
}

export function NewDomain({ opened, close }: NewDomainProps) {
  const [activeStep, setActiveStep] = useState(0);
  const { currentOrganization } = useOrganizations();
  const [newDomain, setNewDomain] = useState<Domain | null>(null);
  const { navigate, dispatch } = useRemails();
  const { projects } = useProjects();

  const domainsApi = `/api/organizations/${currentOrganization?.id}/domains`;

  const { reverifyDomain: verifyDomain, domainVerified, verificationResult } = useVerifyDomain(newDomain);

  const form = useForm<FormValues>({
    initialValues: {
      domain: "",
      project_ids: [],
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

  const save = async (values: FormValues) => {
    const res = await fetch(domainsApi, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ ...values, dkim_key_type: "rsa_sha256" }),
    });
    if (res.status === 201) {
      res.json().then((newDomain) => {
        setNewDomain(newDomain);
        dispatch({ type: "add_domain", domain: newDomain });
        setActiveStep(1);
      });
    } else if (res.status === 409) {
      form.setFieldError("domain", "This domain is already configured");
    } else {
      errorNotification(`Domain ${values.domain} could not be created`);
      console.error(res);
    }
  };

  const deleteDomain = async (domain: Domain | null) => {
    if (!domain) {
      return;
    }

    const res = await fetch(`${domainsApi}/${domain.id}`, {
      method: "DELETE",
    });
    if (res.status !== 200) {
      errorNotification(`Domain ${domain.domain} could not be deleted`);
      console.error(res);
      return;
    }
    dispatch({ type: "remove_domain", domainId: domain.id });
  };

  return (
    <Modal
      opened={opened}
      onClose={() => {
        setActiveStep(0);
        form.reset();
        close();
      }}
      title={
        <Title order={3} component="span">
          Add new domain
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
                label="Domain name"
                key={form.key("domain")}
                value={form.values.domain}
                placeholder="example.com"
                error={form.errors.domain}
                onChange={(event) => form.setFieldValue("domain", event.currentTarget.value)}
              />
              <MultiSelect
                label="Usable by"
                placeholder="any project"
                data={projects.map((p) => ({ value: p.id, label: p.name }))}
                value={form.values.project_ids}
                onChange={(project_ids) => form.setFieldValue("project_ids", project_ids)}
                clearable
                searchable
                nothingFoundMessage="No project found..."
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
              onClick={async () => {
                setActiveStep(0);
                await deleteDomain(newDomain);
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
                navigate("domains.domain", { domain_id: newDomain?.id || "" });
              }}
            >
              Show {newDomain?.domain}
            </Button>
          </Group>
        </Stepper.Step>
      </Stepper>
    </Modal>
  );
}
